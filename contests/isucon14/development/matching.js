// while true; do curl -s http://nginx/api/internal/matching; sleep 0.5; done と同じことをnodejs で実装

const f = () => {
  try {
    fetch("http://localhost:8080/api/internal/matching");
  } catch (e) {}
};

setInterval(f, 500);
