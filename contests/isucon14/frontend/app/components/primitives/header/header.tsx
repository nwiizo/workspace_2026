import { Link } from "@remix-run/react";
import type { PropsWithChildren, FC } from "react";
import { ComponentProps } from "react";
import { twMerge } from "tailwind-merge";

export const Header: FC<
  PropsWithChildren<{ backTo?: `/${string}` } & ComponentProps<"header">>
> = ({ backTo, children, className, ...props }) => {
  return (
    <header
      className={twMerge(
        "p-4 h-18 flex items-center w-full",
        backTo ? "justify-between" : "justify-end",
        className,
      )}
      {...props}
    >
      {backTo && (
        <Link to={backTo} className="hover:underline items-center">
          戻る
        </Link>
      )}
      {children}
    </header>
  );
};
