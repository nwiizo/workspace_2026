/**
 * @param {number} teamId
 * @param {import("playwright-chromium").Page} page
 * @param {string} selector
 */
export const saveScreenshot = async (teamId, page, selector) => {
  const pathname = new URL(page.url()).pathname;
  const filenameBase = pathname.slice(1).replaceAll("/", "_");

  await page.locator(selector).waitFor({ state: "visible" });
  await page.screenshot({
    path: `screeenshots/${teamId}/${filenameBase}.png`,
    fullPage: true,
  });
};
