APP_PORT ?= 18080
APP_URL ?= http://127.0.0.1:$(APP_PORT)
FIXED_APP_PORT ?= 18081
FIXED_APP_URL ?= http://127.0.0.1:$(FIXED_APP_PORT)
ZAP_REPORT ?= reports/zap-full.json
ZAP_SUMMARY ?= reports/zap-findings-summary.md

export APP_PORT
export FIXED_APP_PORT

.PHONY: up up-fixed down logs playwright-screenshots playwright-screenshots-fixed zap-baseline zap-full zap-full-fixed zap-summary zap-summary-fixed clean

up:
	mkdir -p reports
	docker compose up --build -d vulnerable-app
	./scripts/wait-for-http.sh "$(APP_URL)/health"

up-fixed:
	mkdir -p reports
	docker compose up --build -d fixed-app
	./scripts/wait-for-http.sh "$(FIXED_APP_URL)/health"

down:
	docker compose down --remove-orphans

logs:
	docker compose logs -f vulnerable-app

playwright-screenshots: up
	npx --yes playwright screenshot --wait-for-timeout=1000 "$(APP_URL)" reports/playwright-home.png
	npx --yes playwright screenshot --viewport-size=390,844 --wait-for-timeout=1000 "$(APP_URL)" reports/playwright-home-mobile.png
	npx --yes playwright screenshot --wait-for-timeout=1000 "$(APP_URL)/book?id=1%20OR%201=1" reports/playwright-sqli.png
	npx --yes playwright screenshot --wait-for-timeout=1000 "$(APP_URL)/search?q=%3Cstrong%20id%3Dxss-proof%3EReflected%20HTML%3C%2Fstrong%3E" reports/playwright-reflected-html.png

playwright-screenshots-fixed: up-fixed
	npx --yes playwright screenshot --wait-for-timeout=1000 "$(FIXED_APP_URL)" reports/playwright-fixed-home.png
	npx --yes playwright screenshot --wait-for-timeout=1000 "$(FIXED_APP_URL)/book?id=1%20OR%201=1" reports/playwright-fixed-sqli.png
	npx --yes playwright screenshot --wait-for-timeout=1000 "$(FIXED_APP_URL)/search?q=%3Cstrong%20id%3Dxss-proof%3EReflected%20HTML%3C%2Fstrong%3E" reports/playwright-fixed-reflected-html.png

zap-baseline: up
	./scripts/run-zap.sh baseline

zap-full: up
	./scripts/run-zap.sh full

zap-full-fixed: up-fixed
	ZAP_TARGET=http://fixed-app:5000 REPORT_PREFIX=zap-full-fixed ./scripts/run-zap.sh full

zap-summary:
	node scripts/summarize-zap-json.mjs "$(ZAP_REPORT)" > "$(ZAP_SUMMARY)"

zap-summary-fixed:
	node scripts/summarize-zap-json.mjs reports/zap-full-fixed.json > reports/zap-findings-summary-fixed.md

clean:
	rm -rf reports/*
	touch reports/.gitkeep
