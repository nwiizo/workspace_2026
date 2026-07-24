import type { ComponentProps, FC, PropsWithChildren } from "react";
import { twMerge } from "tailwind-merge";

export const ListItem: FC<PropsWithChildren<ComponentProps<"li">>> = ({
  children,
  className,
  ...props
}) => {
  return (
    <li className={twMerge("border-b", className)} {...props}>
      {children}
    </li>
  );
};
