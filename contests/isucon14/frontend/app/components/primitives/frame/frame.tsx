import type { ComponentProps, FC, PropsWithChildren } from "react";
import { twMerge } from "tailwind-merge";

export const MainFrame: FC<PropsWithChildren<ComponentProps<"div">>> = ({
  children,
  className,
  ...props
}) => {
  return (
    <main
      className={twMerge(
        "md:max-w-screen-md h-full relative ml-auto mr-auto shadow-xl bg-white flex flex-col",
        className,
      )}
      {...props}
    >
      <div className="flex flex-col min-h-screen">{children}</div>
    </main>
  );
};
