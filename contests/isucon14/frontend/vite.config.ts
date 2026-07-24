import { vitePlugin as remix } from "@remix-run/dev";
import { createHash } from "crypto";
import { fdir } from "fdir";
import { existsSync, readFileSync, writeFileSync } from "fs";
import { readFile, writeFile } from "fs/promises";
import path, { join } from "path";
import { defineConfig, type Plugin, type UserConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";
import {
  AppPostUsersRequestBody,
  ChairPostChairsRequestBody,
  OwnerPostOwnersRequestBody,
} from "~/api/api-components";
import { alternativeURLExpression } from "./api-url.mjs";

const DEFAULT_HOSTNAME = "localhost";
const DEFAULT_PORT = 3000;

const DEFAULT_URL = `http://${DEFAULT_HOSTNAME}:${DEFAULT_PORT}`;

type APIResponse = Record<string, string>;

const intialOwnerData = existsSync("./initial-data.json")
  ? (JSON.parse(readFileSync("./initial-data.json").toString()) as unknown)
  : undefined;

const getLoggedInURLForClient = async () => {
  const generateURL = (r: APIResponse) => {
    const id: string = r["id"];
    const accessToken: string = r["access_token"];
    return `${DEFAULT_URL}/client?access_token=${accessToken}&id=${id}`;
  };

  if (existsSync(`./client.login-cache.json`)) {
    return generateURL(
      JSON.parse(
        readFileSync(`./client.login-cache.json`).toString(),
      ) as APIResponse,
    );
  }

  const response = await fetch("http://localhost:8080/api/app/users", {
    body: JSON.stringify({
      username: "testIsuconUser",
      firstname: "isucon",
      lastname: "isucon",
      date_of_birth: "11111111",
    } satisfies AppPostUsersRequestBody),
    method: "POST",
  });
  const json = (await response.json()) as APIResponse;

  writeFileSync(`./client.login-cache.json`, JSON.stringify(json));
  console.log("writeFileSync!", json);
  return generateURL(json);
};

const getLoggedInURLForDriver = async () => {
  const generateURL = (r: APIResponse) => {
    const id: string = r["id"];
    const accessToken: string = r["access_token"];
    return `${DEFAULT_URL}/driver?access_token=${accessToken}&id=${id}`;
  };

  if (existsSync(`./driver.login-cache.json`)) {
    return generateURL(
      JSON.parse(
        readFileSync(`./driver.login-cache.json`).toString(),
      ) as APIResponse,
    );
  }

  // POST /provider/register => POST /chair/register
  const providerResponse = await fetch(
    "http://localhost:8080/api/owner/ownsers",
    {
      body: JSON.stringify({
        name: "isuconProvider",
      } satisfies OwnerPostOwnersRequestBody),
      method: "POST",
    },
  );
  const providerJSON = (await providerResponse.json()) as Record<
    string,
    string
  >;
  const response = await fetch("http://localhost:8080/chair/register", {
    body: JSON.stringify({
      name: "isuconChair001",
      model: "isuconChair",
      chair_register_token: providerJSON["chair_register_token"],
    } satisfies ChairPostChairsRequestBody),
    method: "POST",
  });
  const json = (await response.json()) as APIResponse;

  writeFileSync(`./driver.login-cache.json`, JSON.stringify(json));
  console.log("writeFileSync!", json);
  return generateURL(json);
};

const customConsolePlugin: Plugin = {
  name: "custom-test-user-login",
  configureServer(server) {
    server.httpServer?.once("listening", () => {
      (async () => {
        console.log(
          `logined client page: \x1b[32m  ${await getLoggedInURLForClient()} \x1b[0m`,
        );
        console.log(
          `logined driver page: \x1b[32m  ${await getLoggedInURLForDriver()} \x1b[0m`,
        );
      })().catch((e) => console.log(`LOGIN ERROR: ${e}`));
    });
  },
};

const generateHashesFile = (): Plugin => {
  const clientOutputDirectory = path.resolve(__dirname, "./build/client");
  const benchRoot = path.resolve(__dirname, "../bench");

  type RouteInformation = {
    css: string[];
    imports: string[];
    module: string;
    parentId?: string;
    path?: string;
  };
  const getAllFilesFromSingleRoute = (route: RouteInformation) => [
    route.module,
    ...route.imports,
    ...route.css,
  ];
  const getAllFilesFromRouteIncludingAncestors = (
    route: RouteInformation,
    manifestRoutes: Record<string, RouteInformation>,
  ): string[] => [
    ...getAllFilesFromSingleRoute(route),
    ...(route.parentId
      ? getAllFilesFromRouteIncludingAncestors(
          manifestRoutes[route.parentId],
          manifestRoutes,
        )
      : []),
  ];
  const unique = <T>(arr: T[]) => [...new Set(arr)];

  return {
    name: "generate-hashes-file",
    apply(_config, { isSsrBuild }) {
      return !!isSsrBuild;
    },
    enforce: "post",
    writeBundle: {
      order: "post",
      sequential: true,
      handler: async () => {
        const files = await new fdir()
          .withRelativePaths()
          .crawl(clientOutputDirectory)
          .withPromise();
        const hashes = await Promise.all(
          files
            .filter((file) => file !== ".vite/manifest.json")
            .map(async (file) => {
              const hash = createHash("md5");
              hash.update(await readFile(join(clientOutputDirectory, file)));
              return [file, hash.digest("hex")];
            }),
        );
        await writeFile(
          join(benchRoot, "./benchrun/frontend_hashes.json"),
          JSON.stringify(Object.fromEntries(hashes)),
        );

        const manifestFileName = files.find((file) =>
          file.startsWith("assets/manifest-"),
        );
        if (!manifestFileName) throw new Error("manifest file not found");
        const manifestFile = await readFile(
          join(clientOutputDirectory, manifestFileName),
          "utf8",
        );
        if (!manifestFile.includes("window.__remixManifest"))
          throw new Error("different manifest file found");

        const manifestFileContent = (0, eval)(
          "Object.assign(" +
            manifestFile
              .replace(/^window\.__remixManifest=/, "")
              .replace(/;$/, "") +
            ")",
        ) as {
          entry: RouteInformation;
          routes: Record<string, RouteInformation>;
        };
        const favicons = files
          .filter(
            (file) => file === "favicon.ico" || file === "favicon-32x32.png",
          )
          .map((f) => `/${f}`);
        const assetsForMap = files
          .filter(
            (file) =>
              file.startsWith("images/buildings") ||
              file.startsWith("images/house") ||
              file.startsWith("images/town"),
          )
          .map((f) => `/${f}`);
        const modulesForEachPath = Object.fromEntries(
          Object.values(manifestFileContent.routes)
            .filter((route) => "path" in route)
            .map(
              (route) =>
                [
                  "/" + route.path!,
                  unique([
                    ...getAllFilesFromSingleRoute(manifestFileContent.entry),
                    ...getAllFilesFromRouteIncludingAncestors(
                      route,
                      manifestFileContent.routes,
                    ),
                    ...favicons,
                    ...(route.path === "client" ? assetsForMap : []),
                  ]),
                ] as [string, string[]],
            ),
        );
        await writeFile(
          join(benchRoot, "./benchrun/frontend_files.json"),
          JSON.stringify(modulesForEachPath),
        );
      },
    },
  };
};

export const config = {
  plugins: [
    remix({
      ssr: false,
      future: {
        v3_fetcherPersist: true,
        v3_relativeSplatPath: true,
        v3_throwAbortReason: true,
      },
    }),
    tsconfigPaths(),
    customConsolePlugin,
    generateHashesFile(),
  ],
  define: {
    [alternativeURLExpression]: `"${process.env["API_BASE_URL"] ?? "."}"`,
    __INITIAL_DATA__: intialOwnerData,
  },
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:8080",
        changeOrigin: true,
      },
    },
    host: DEFAULT_HOSTNAME,
    port: DEFAULT_PORT,
    strictPort: true,
  },
  preview: {
    proxy: {
      "/api": {
        target: "http://localhost:8080",
        changeOrigin: true,
      },
    },
    host: DEFAULT_HOSTNAME,
    port: DEFAULT_PORT,
    strictPort: true,
  },
} as const satisfies UserConfig;

export default defineConfig(config);
