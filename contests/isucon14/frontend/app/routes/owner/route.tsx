import { Link, Outlet, useMatches, useNavigate } from "@remix-run/react";
import { twMerge } from "tailwind-merge";
import colors from "tailwindcss/colors";
import { AccountSwitchIcon } from "~/components/icon/account-switch";
import { IsurideIcon } from "~/components/icon/isuride";
import { Button } from "~/components/primitives/button/button";
import { OwnerProvider } from "~/contexts/owner-context";

const tabs = [
  { key: "index", label: "椅子", to: "/owner" },
  { key: "sales", label: "売上", to: "/owner/sales" },
] as const;

const Content = () => {
  const matches = useMatches();
  const activeTab = matches[2]?.pathname.split("/").at(-1) || "index";

  return (
    <nav className="flex after:w-full after:border-b after:border-gray-300">
      <ul className="flex shrink-0">
        {tabs.map((tab) => (
          <li
            key={tab.key}
            className={twMerge([
              "rounded-tl-md rounded-tr-md",
              tab.key === activeTab
                ? "border border-b-transparent"
                : "border-l-transparent border-t-transparent border-r-transparent border",
            ])}
          >
            <Link to={tab.to} className="block px-8 py-3">
              {tab.label}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
};

export default function OwnerLayout() {
  const navigate = useNavigate();

  return (
    <OwnerProvider>
      <div className="bg-neutral-100 flex justify-center">
        <div className="p-6 lg:p-10 h-screen flex flex-col overflow-x-hidden w-full max-w-6xl bg-white">
          <div className="flex items-center justify-between mb-6">
            <h1 className="flex items-baseline text-xl lg:text-3xl">
              <IsurideIcon
                className="relative top-[2px] mr-2"
                width={40}
                height={40}
              />
              オーナー様向け管理画面
            </h1>
            <Button
              size="sm"
              className="flex items-center justify-center "
              onClick={() => navigate("/owner/login")}
            >
              <AccountSwitchIcon className="me-1" fill={colors.neutral[600]} />
              アカウント切替え
            </Button>
          </div>
          <Content />
          <div className="flex-1 overflow-auto pt-8 pb-16 max-w-7xl xl:flex justify-center">
            <Outlet />
          </div>
        </div>
      </div>
    </OwnerProvider>
  );
}
