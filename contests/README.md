# Contests

Programming contests and security challenges.

## Contents

| Directory | Description | Status |
|-----------|-------------|--------|
| `juice_shop/` | OWASP Juice Shop CTF - Web security challenges | Active |

## OWASP Juice Shop

### Quick Start

```bash
# Start Docker runtime (macOS with Colima)
colima start

# Run Juice Shop
docker run -d -p 3000:3000 --name juice-shop bkimminich/juice-shop

# Open in browser
open http://localhost:3000
```

### Management

```bash
# Check status
docker ps --filter name=juice-shop

# Stop
docker stop juice-shop

# Start (after stop)
docker start juice-shop

# Remove and reset
docker rm -f juice-shop
docker run -d -p 3000:3000 --name juice-shop bkimminich/juice-shop
```

### Access Points

| URL | Description |
|-----|-------------|
| http://localhost:3000 | Main application |
| http://localhost:3000/#/score-board | Challenge scoreboard |
| http://localhost:3000/#/administration | Admin panel (requires login) |

## Adding New Contests

Create a subdirectory for each contest with:
- `README.md` - Contest overview, rules, and tools
- `CLAUDE.md` - Project-specific instructions (optional)
