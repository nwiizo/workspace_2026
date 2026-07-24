import type { ComponentProps, FC, PropsWithChildren } from "react";
import { twMerge } from "tailwind-merge";

export const ConfigFrame: FC<PropsWithChildren<ComponentProps<"div">>> = ({
  children,
  className,
  ...props
}) => {
  return (
    <div
      className={twMerge(
        "bg-white rounded shadow px-6 py-4 w-full relative",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
};
