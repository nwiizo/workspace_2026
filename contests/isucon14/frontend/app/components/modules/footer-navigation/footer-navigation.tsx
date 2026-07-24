import { NavLink } from "@remix-run/react";
import type { ComponentProps, FC } from "react";
import { AccountIcon } from "~/components/icon/account";
import { HistoryIcon } from "~/components/icon/history";
import { IsurideIcon } from "~/components/icon/isuride";

type NavigationMenu = {
  link: `/${string}`;
  label: string;
  icon: FC<ComponentProps<"svg">>;
};

const MenuList = [
  {
    icon: IsurideIcon,
    link: "/client",
    label: "RIDE",
  },
  {
    icon: HistoryIcon,
    link: "/client/history",
    label: "HISTORY",
  },
  {
    icon: AccountIcon,
    link: "/client/user",
    label: "USER",
  },
] as const satisfies NavigationMenu[];

export const FooterNavigation: FC = () => {
  return (
    <nav className="sticky bottom-[env(safe-area-inset-bottom)] z-10 border-t border-secondary-border bg-white">
      <ul className="flex justify-around mx-10">
        {MenuList.map((menu) => (
          <li
            key={menu.link}
            className="flex justify-center border-b-4 border-transparent has-[.active]:border-black"
          >
            <NavLink
              to={menu.link}
              end
              className={({ isActive }) =>
                `flex flex-col items-center justify-center gap-1 px-4 py-1.5 text-xs hover:bg-secondary-hover ${isActive ? "pointer-events-none active" : ""}`
              }
            >
              <menu.icon className="fill-neutral-950" width={30} height={30} />
              {menu.label}
            </NavLink>
          </li>
        ))}
      </ul>
    </nav>
  );
};
