import { defineConfig } from "@openapi-codegen/cli";
import {
  generateReactQueryComponents,
  generateSchemaTypes,
} from "@openapi-codegen/typescript";
import { ConfigBase } from "@openapi-codegen/typescript/lib/generators/types";
import { readFile, readdir, writeFile } from "fs/promises";
import { join as pathJoin } from "path";
import {
  alternativeAPIURLString,
  alternativeURLExpression,
} from "./api-url.mjs";

const outputDir = "./app/api";

export default defineConfig({
  isucon: {
    from: {
      relativePath: "../webapp/openapi.yaml",
      source: "file",
    },
    outputDir,
    to: async (context) => {
      /**
       * openapi.yamlに定義済みのurl配列
       */
      const targetBaseCandidateURLs = context.openAPIDocument.servers?.map(
        (server) => server.url,
      );
      if (
        targetBaseCandidateURLs === undefined ||
        targetBaseCandidateURLs.length === 0
      ) {
        throw Error("must define servers.url");
      }
      if (targetBaseCandidateURLs.length > 1) {
        throw Error("he servers.url must have only one entry.");
      }

      const contextServers = context.openAPIDocument.servers;
      context.openAPIDocument.servers = contextServers?.map((serverObject) => {
        return {
          ...serverObject,
          url: alternativeAPIURLString,
        };
      });

      const configBase: ConfigBase = {
        filenamePrefix: "api",
        filenameCase: "kebab",
      };
      const { schemasFiles } = await generateSchemaTypes(context, configBase);
      await generateReactQueryComponents(context, {
        ...configBase,
        schemasFiles,
      });

      /**
       * viteのdefineで探索可能にする
       */
      await rewriteFileInTargetDir(outputDir, (content) =>
        content.replace(
          `"${alternativeAPIURLString}"`,
          alternativeURLExpression,
        ),
      );
      /**
       * SSE通信などでは、自動生成のfetcherを利用しないため
       */
      await writeFile(
        `${outputDir}/${configBase.filenamePrefix}-base-url.ts`,
        `export const apiBaseURL = ${alternativeURLExpression};\n`,
      );
    },
  },
});

type RewriteFn = (content: string) => string;

/**
 * 指定されたディレクトリ配下のファイルコンテンツをrewriteFnで置き換える
 */
async function rewriteFileInTargetDir(
  dirPath: string,
  rewriteFn: RewriteFn,
): Promise<void> {
  try {
    const files = await readdir(dirPath, { withFileTypes: true });
    for (const file of files) {
      const filePath = pathJoin(dirPath, file.name);
      if (file.isDirectory()) {
        await rewriteFileInTargetDir(filePath, rewriteFn);
        continue;
      }
      if (file.isFile()) {
        await rewriteFile(filePath, rewriteFn);
      }
    }
  } catch (err) {
    if (typeof err === "string") {
      console.error(`CONSOLE ERROR: ${err}`);
    }
  }
}

async function rewriteFile(filePath: string, rewriteFn: RewriteFn) {
  const data = await readFile(filePath, "utf8");
  const rewrittenContent = rewriteFn(data);
  await writeFile(filePath, rewrittenContent);
}
