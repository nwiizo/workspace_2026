use crate::{
    Result,
    neural::{Activation, Mlp},
    random::Rng,
};
#[derive(Clone, Copy)]
pub struct Transition {
    pub next: usize,
    pub reward: f64,
    pub terminal: bool,
}
pub struct Mdp {
    pub actions: Vec<Vec<Transition>>,
}
impl Mdp {
    fn validate(&self, start: usize) -> Result<()> {
        if start >= self.actions.len()
            || self.actions[start].is_empty()
            || self.actions.iter().flatten().any(|t| {
                t.next >= self.actions.len()
                    || !t.reward.is_finite()
                    || (!t.terminal && self.actions[t.next].is_empty())
            })
        {
            return Err("invalid MDP");
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub struct TdConfig {
    pub episodes: usize,
    pub max_steps: usize,
    pub alpha: f64,
    pub gamma: f64,
    pub epsilon: f64,
}
impl TdConfig {
    fn validate(&self) -> Result<()> {
        if self.episodes == 0
            || self.max_steps == 0
            || !(0.0..=1.0).contains(&self.alpha)
            || self.alpha == 0.0
            || !(0.0..1.0).contains(&self.gamma)
            || !(0.0..=1.0).contains(&self.epsilon)
        {
            return Err("invalid TD settings");
        }
        Ok(())
    }
}
pub fn epsilon_greedy(q: &[f64], epsilon: f64, rng: &mut Rng) -> Result<usize> {
    if q.is_empty() || q.iter().any(|v| !v.is_finite()) || !(0.0..=1.0).contains(&epsilon) {
        return Err("invalid action values");
    }
    if rng.uniform() < epsilon {
        return rng.index(q.len()).ok_or("no actions");
    }
    let max = q.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let ties: Vec<_> = q
        .iter()
        .enumerate()
        .filter(|(_, v)| **v == max)
        .map(|(i, _)| i)
        .collect();
    Ok(ties[rng.index(ties.len()).expect("at least one maximum")])
}
pub fn sarsa_update(current: f64, reward: f64, next: f64, alpha: f64, gamma: f64) -> Result<f64> {
    if [current, reward, next].iter().any(|v| !v.is_finite())
        || !(0.0..=1.0).contains(&alpha)
        || !(0.0..=1.0).contains(&gamma)
    {
        return Err("invalid SARSA update");
    }
    Ok(current + alpha * (reward + gamma * next - current))
}
pub fn train_td(
    mdp: &Mdp,
    start: usize,
    config: TdConfig,
    sarsa: bool,
    seed: u64,
) -> Result<Vec<Vec<f64>>> {
    mdp.validate(start)?;
    config.validate()?;
    let mut rng = Rng::new(seed);
    let mut q: Vec<_> = mdp.actions.iter().map(|a| vec![0.0; a.len()]).collect();
    for _ in 0..config.episodes {
        let mut state = start;
        let mut action = epsilon_greedy(&q[state], config.epsilon, &mut rng)?;
        for _ in 0..config.max_steps {
            let transition = mdp.actions[state][action];
            let next_action = if transition.terminal {
                0
            } else {
                epsilon_greedy(&q[transition.next], config.epsilon, &mut rng)?
            };
            let future = if transition.terminal {
                0.0
            } else if sarsa {
                q[transition.next][next_action]
            } else {
                q[transition.next]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
            };
            q[state][action] = sarsa_update(
                q[state][action],
                transition.reward,
                future,
                config.alpha,
                config.gamma,
            )?;
            if transition.terminal {
                break;
            }
            state = transition.next;
            action = next_action;
        }
    }
    Ok(q)
}
pub fn value_iteration(
    mdp: &Mdp,
    gamma: f64,
    tolerance: f64,
    iterations: usize,
) -> Result<Vec<f64>> {
    if mdp.actions.is_empty()
        || !(0.0..1.0).contains(&gamma)
        || !tolerance.is_finite()
        || tolerance <= 0.0
    {
        return Err("invalid value iteration settings");
    }
    if mdp
        .actions
        .iter()
        .flatten()
        .any(|t| t.next >= mdp.actions.len() || !t.reward.is_finite())
    {
        return Err("invalid transition");
    }
    let mut values = vec![0.0; mdp.actions.len()];
    for _ in 0..iterations {
        let next: Vec<_> = mdp
            .actions
            .iter()
            .map(|actions| {
                actions
                    .iter()
                    .map(|a| {
                        a.reward
                            + if a.terminal {
                                0.0
                            } else {
                                gamma * values[a.next]
                            }
                    })
                    .reduce(f64::max)
                    .unwrap_or(0.0)
            })
            .collect();
        let change = next
            .iter()
            .zip(&values)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        values = next;
        if change < tolerance {
            break;
        }
    }
    Ok(values)
}
/// Neural Q learning with replay and periodically copied target weights.
pub fn train_dqn(mdp: &Mdp, start: usize, config: TdConfig, seed: u64) -> Result<Mlp> {
    mdp.validate(start)?;
    config.validate()?;
    let action_count = mdp.actions[start].len();
    let states = mdp.actions.len();
    if mdp
        .actions
        .iter()
        .any(|a| !a.is_empty() && a.len() != action_count)
    {
        return Err("DQN requires fixed action count");
    }
    let encode = |s: usize| {
        let mut v = vec![0.0; states];
        v[s] = 1.0;
        v
    };
    let mut model = Mlp::new(
        &[states, 8, action_count],
        Activation::Tanh,
        Activation::Linear,
        seed,
    )?;
    let mut target = model.clone();
    let mut rng = Rng::new(seed);
    let mut replay = Vec::new();
    let mut updates = 0;
    for _ in 0..config.episodes {
        let mut state = start;
        for _ in 0..config.max_steps {
            let values = model.predict(&encode(state))?;
            let action = epsilon_greedy(&values, config.epsilon, &mut rng)?;
            let tr = mdp.actions[state][action];
            if replay.len() == 128 {
                replay.remove(0);
            }
            replay.push((state, action, tr));
            for _ in 0..4 {
                let (s, a, t) = replay[rng.index(replay.len()).expect("nonempty")];
                let row = encode(s);
                let mut labels = model.predict(&row)?;
                labels[a] = t.reward
                    + if t.terminal {
                        0.0
                    } else {
                        config.gamma
                            * target
                                .predict(&encode(t.next))?
                                .into_iter()
                                .fold(f64::NEG_INFINITY, f64::max)
                    };
                model.train_mse(&[row], &[labels], 1, config.alpha)?;
                updates += 1;
                if updates % 32 == 0 {
                    target = model.clone();
                }
            }
            if tr.terminal {
                break;
            }
            state = tr.next;
        }
    }
    Ok(model)
}
