import type { MetaFunction } from "@remix-run/node";
import { Link } from "@remix-run/react";
import { twMerge } from "tailwind-merge";
import colors from "tailwindcss/colors";
import { DesktopIcon } from "~/components/icon/desktop";
import { IsurideIcon } from "~/components/icon/isuride";
import { MobileIcon } from "~/components/icon/mobile";
import { MainFrame } from "~/components/primitives/frame/frame";
import { Text } from "~/components/primitives/text/text";

export const meta: MetaFunction = () => {
  return [
    { title: "Top | ISURIDE" },
    {
      name: "description",
      content: "ISURIDEは椅子でユーザーを運ぶ新感覚のサービスです",
    },
  ];
};

const Links = [
  {
    to: "/simulator",
    title: "Simulator Aplication",
    description: "ISUCON競技者用 アプリ動作シミュレーター",
    Icon: () => <IsurideIcon fill="#fff" width={30} height={30}></IsurideIcon>,
    style: "bg-sky-600 text-white hover:bg-sky-700",
  },
  {
    to: "/client",
    title: "Client Application",
    description: "ISURIDE利用者用 モバイルクラインアント",
    Icon: () => (
      <MobileIcon width={40} height={40} fill={colors.neutral[800]} />
    ),
    style: "",
  },
  {
    to: "/owner/login",
    title: "Owner Application",
    description: "ISURIDEオーナー向け 管理アプリケーション",
    Icon: () => (
      <DesktopIcon width={40} height={40} fill={colors.neutral[800]} />
    ),
    style: "",
  },
] as const;

export default function Index() {
  return (
    <MainFrame>
      <img
        src="/images/top-bg.png"
        alt=""
        role="presentation"
        className="absolute top-0 left-0 w-full opacity-90"
      />
      <div className="relative z-10">
        <h1 className="mt-[35%] flex justify-center w-full mb-32">
          <img
            src="/images/top-logo.svg"
            alt=""
            role="presentation"
            style={{ aspectRatio: 544 / 140 }}
            className="max-w-80 w-full px-4"
          />
          <span className="sr-only">ISURIDE Top</span>
        </h1>
        <ul className="space-y-8">
          {Links.map(({ to, title, description, Icon, style }) => {
            return (
              <li key={to} className="">
                <Link
                  to={to}
                  className={twMerge(
                    "flex justify-center items-center flex-row space-x-4",
                    "px-10 py-4 bg-neutral-100 rounded-full mx-auto w-[300px] h-[100px] space-y-1 shadow-md hover:bg-neutral-200 transition-colors",
                    "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-900",
                    style,
                  )}
                >
                  <Icon />
                  <p className="space-y-1 flex flex-col">
                    <Text tagName="span" bold>
                      {title}
                    </Text>
                    <Text
                      tagName="span"
                      size="xs"
                      className="break-keep leading-normal"
                    >
                      {description}
                    </Text>
                  </p>
                </Link>
              </li>
            );
          })}
        </ul>
      </div>
    </MainFrame>
  );
}
