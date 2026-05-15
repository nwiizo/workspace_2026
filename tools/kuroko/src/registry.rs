//! Service registry — holds every registered service and exposes lookups by
//! protocol dispatcher.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::service::{
    CborProtocolService, DynCborService, DynJsonService, DynQueryService, DynService,
    JsonProtocolService, QueryProtocolService,
};

#[derive(Default)]
pub struct Registry {
    inner: RwLock<RegistryInner>,
}

#[derive(Default)]
struct RegistryInner {
    services: HashMap<&'static str, DynService>,
    /// `X-Amz-Target` prefix → service handler.
    json_by_prefix: HashMap<&'static str, DynJsonService>,
    /// Action name → list of (sdk_id, service). Multiple Query services share
    /// action names (e.g. `DescribeDBInstances` exists in `rds` and `neptune`),
    /// so we disambiguate by SDK service identifier sent via User-Agent.
    query_by_action: HashMap<&'static str, Vec<DynQueryService>>,
    /// Smithy service name → CBOR handler.
    cbor_by_service: HashMap<&'static str, DynCborService>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, svc: DynService) {
        let mut inner = self.inner.write();
        inner.services.insert(svc.name(), svc);
    }

    pub fn register_json(&self, svc: Arc<dyn JsonProtocolService>) {
        let mut inner = self.inner.write();
        let base: DynService = svc.clone();
        inner.services.insert(svc.name(), base);
        inner.json_by_prefix.insert(svc.target_prefix(), svc);
    }

    pub fn register_query(&self, svc: Arc<dyn QueryProtocolService>) {
        let mut inner = self.inner.write();
        let base: DynService = svc.clone();
        inner.services.insert(svc.name(), base);
        for action in svc.actions() {
            inner
                .query_by_action
                .entry(*action)
                .or_default()
                .push(svc.clone());
        }
    }

    pub fn register_cbor(&self, svc: Arc<dyn CborProtocolService>) {
        let mut inner = self.inner.write();
        let base: DynService = svc.clone();
        inner.services.insert(svc.name(), base);
        inner.cbor_by_service.insert(svc.smithy_service(), svc);
    }

    pub fn get(&self, name: &str) -> Option<DynService> {
        self.inner.read().services.get(name).cloned()
    }

    pub fn all(&self) -> Vec<DynService> {
        self.inner.read().services.values().cloned().collect()
    }

    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.inner.read().services.keys().copied().collect();
        names.sort();
        names
    }

    pub fn json_for_target(&self, target: &str) -> Option<(DynJsonService, String)> {
        // X-Amz-Target is "<TargetPrefix>.<Action>". Split on the last '.'.
        let dot = target.rfind('.')?;
        let prefix = &target[..dot];
        let action = &target[dot + 1..];
        let inner = self.inner.read();
        inner
            .json_by_prefix
            .get(prefix)
            .cloned()
            .map(|svc| (svc, action.to_string()))
    }

    pub fn query_for_action(
        &self,
        action: &str,
        sdk_id_hint: Option<&str>,
    ) -> Option<DynQueryService> {
        let inner = self.inner.read();
        let candidates = inner.query_by_action.get(action)?;
        if let Some(hint) = sdk_id_hint
            && let Some(svc) = candidates
                .iter()
                .find(|s| s.sdk_id().eq_ignore_ascii_case(hint))
        {
            return Some(svc.clone());
        }
        candidates.first().cloned()
    }

    pub fn cbor_for_service(&self, smithy: &str) -> Option<DynCborService> {
        self.inner.read().cbor_by_service.get(smithy).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::Service;
    use async_trait::async_trait;

    struct Dummy;
    #[async_trait]
    impl Service for Dummy {
        fn name(&self) -> &'static str {
            "dummy"
        }
    }

    #[test]
    fn register_and_lookup() {
        let reg = Registry::new();
        reg.register(Arc::new(Dummy));
        assert_eq!(reg.names(), vec!["dummy"]);
        assert!(reg.get("dummy").is_some());
        assert!(reg.get("missing").is_none());
    }
}
