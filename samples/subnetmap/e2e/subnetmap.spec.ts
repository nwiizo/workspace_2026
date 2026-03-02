import { test, expect } from "@playwright/test";

test.describe("SubnetMap E2E Tests", () => {
  test("dashboard loads correctly", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("h1")).toContainText("Dashboard");
    await expect(page.locator("text=Total Subnets")).toBeVisible();
    await expect(page.locator("text=Total IPs")).toBeVisible();
    await expect(page.getByRole("heading", { name: "Active Alerts" })).toBeVisible();
    await page.screenshot({ path: "screenshots/dashboard.png", fullPage: true });
  });

  test("subnet list page", async ({ page }) => {
    await page.goto("/subnets");
    await expect(page.locator("h1")).toContainText("Subnets");
    await expect(page.locator("text=Add Subnet")).toBeVisible();
    await page.screenshot({ path: "screenshots/subnet-list-empty.png", fullPage: true });
  });

  test("create a subnet", async ({ page }) => {
    await page.goto("/subnets");
    await page.click("text=Add Subnet");
    await page.fill('input[placeholder="192.168.1.0/24"]', "10.0.0.0/24");
    await page.fill('input[placeholder="Production Network"]', "Office Network");
    await page.fill('input[placeholder="Optional description"]', "Main office subnet");
    await page.click("text=Create Subnet");
    await page.waitForTimeout(1000);
    await page.screenshot({ path: "screenshots/subnet-created.png", fullPage: true });
  });

  test("ip addresses page", async ({ page }) => {
    await page.goto("/ips");
    await expect(page.locator("h1")).toContainText("IP Addresses");
    await expect(page.locator("text=All")).toBeVisible();
    await expect(page.locator("text=Assigned")).toBeVisible();
    await page.screenshot({ path: "screenshots/ip-list.png", fullPage: true });
  });

  test("vlan list page", async ({ page }) => {
    await page.goto("/vlans");
    await expect(page.locator("h1")).toContainText("VLANs");
    await page.screenshot({ path: "screenshots/vlan-list.png", fullPage: true });
  });

  test("create a VLAN", async ({ page }) => {
    await page.goto("/vlans");
    await page.click("text=Add VLAN");
    await page.fill('input[placeholder="100"]', "100");
    await page.fill('input[placeholder="Management"]', "Management VLAN");
    await page.fill('input[placeholder="Optional"]', "Network management");
    await page.click("text=Create VLAN");
    await page.waitForTimeout(1000);
    await page.screenshot({ path: "screenshots/vlan-created.png", fullPage: true });
  });

  test("audit log page", async ({ page }) => {
    await page.goto("/audit");
    await expect(page.locator("h1")).toContainText("Audit Log");
    await page.screenshot({ path: "screenshots/audit-log.png", fullPage: true });
  });

  test("search page", async ({ page }) => {
    await page.goto("/search");
    await expect(page.locator("h1")).toContainText("Search");
    await expect(
      page.locator("text=Enter a search term")
    ).toBeVisible();
    await page.screenshot({ path: "screenshots/search-empty.png", fullPage: true });
  });

  test("settings page with tags", async ({ page }) => {
    await page.goto("/settings");
    await expect(page.locator("h1")).toContainText("Settings");
    await expect(page.getByRole("heading", { name: "Tags" })).toBeVisible();
    await page.screenshot({ path: "screenshots/settings.png", fullPage: true });
  });

  test("create a tag", async ({ page }) => {
    await page.goto("/settings");
    await page.click("text=Add Tag");
    await page.fill('input[placeholder="production"]', "production");
    await page.click("text=Create");
    await page.waitForTimeout(1000);
    await page.screenshot({ path: "screenshots/tag-created.png", fullPage: true });
  });

  test("navigation sidebar links work", async ({ page }) => {
    await page.goto("/");

    // Test sidebar navigation
    await page.click("text=Subnets");
    await expect(page).toHaveURL("/subnets");

    await page.click("text=IP Addresses");
    await expect(page).toHaveURL("/ips");

    await page.click("text=VLANs");
    await expect(page).toHaveURL("/vlans");

    await page.click("text=Audit Log");
    await expect(page).toHaveURL("/audit");

    await page.click("text=Search");
    await expect(page).toHaveURL("/search");

    await page.click("text=Dashboard");
    await expect(page).toHaveURL("/");

    await page.screenshot({ path: "screenshots/navigation-test.png", fullPage: true });
  });
});
