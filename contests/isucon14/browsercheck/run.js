import fs from "node:fs/promises";
import { chromium } from "playwright-chromium";
import { baseUrl, pages, randomString, teams } from "./config.js";
import { saveScreenshot } from "./utils.js";

if (await fs.stat("screeenshots").catch(() => null)) {
  await fs.rm("screeenshots", { recursive: true });
}

for (const { teamId, ip } of teams) {
  console.log("Checking teamId:", teamId);
  const browser = await chromium.launch({
    args: [`--host-resolver-rules=MAP isuride.xiv.isucon.net ${ip}:443`],
  });

  const page = await browser.newPage();
  // register on owner
  {
    await page.goto(`${baseUrl}/owner/register`);
    await saveScreenshot(teamId, page, 'input[name="ownerName"]');

    await page
      .locator('input[name="ownerName"]')
      .fill(`isucon_${randomString}`);
    const ownerPagePromise = page.waitForURL(`${baseUrl}/owner`);
    await page.locator("button", { hasText: "登録" }).click();
    await ownerPagePromise;

    page.context().clearCookies(); // logout
  }
  // register on client page
  {
    await page.goto(`${baseUrl}/client/register`);
    await saveScreenshot(teamId, page, 'form[action="/client/register"]');

    await page.locator('input[name="username"]').fill(`isucon_${randomString}`);
    await page.locator('input[name="lastname"]').fill("椅子田");
    await page.locator('input[name="firstname"]').fill("譲之助");
    await page.locator('input[name="date_of_birth"]').fill("1980-01-01");
    const registerTokenPagePromise = page.waitForURL(
      `${baseUrl}/client/register-payment`
    );
    await page.locator('button[type="submit"]').click();
    await registerTokenPagePromise;

    await saveScreenshot(
      teamId,
      page,
      'form[action="/client/register-payment"]'
    );
    await page.locator('input[name="payment-token"]').fill("isucon");
    await page.locator('button[type="submit"]').click();
  }
  // login on owner page
  {
    await page.goto(`${baseUrl}/owner/login`);
    await saveScreenshot(teamId, page, 'input[name="sessionToken"]');

    await page.locator('details').click();
    await page.locator('button', { hasText: 'Seat Revival' }).click()

    const ownerPagePromise = page.waitForURL(`${baseUrl}/owner`);
    await page.locator("button", { hasText: "ログイン" }).click();
    await ownerPagePromise;
  }

  for (const { path, selector } of pages) {
    await page.goto(`${baseUrl}${path}`);
    await saveScreenshot(teamId, page, selector);
  }

  await browser.close();
}
