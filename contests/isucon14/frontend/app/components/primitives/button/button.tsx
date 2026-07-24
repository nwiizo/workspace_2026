import { ComponentProps, FC, PropsWithChildren, useMemo } from "react";
import { twMerge } from "tailwind-merge";

type Variant = "light" | "primary" | "skelton";
type Size = "sm" | "md";

export const Button: FC<
  PropsWithChildren<
    ComponentProps<"button"> & { variant?: Variant; size?: Size }
  >
> = ({
  children,
  className,
  variant = "light",
  size = "md",
  disabled,
  ...props
}) => {
  const variantClasses = useMemo(() => {
    switch (variant) {
      case "primary":
        return "text-white bg-sky-700";
      case "light":
        return "bg-[#F0EFED]";
      default:
        return;
    }
  }, [variant]);

  const sizeClasses = useMemo(() => {
    switch (size) {
      case "sm":
        return "py-2 px-3";
      case "md":
        return "py-3.5 px-4";
    }
  }, [size]);

  return (
    <button
      type="button"
      className={twMerge(
        "text-center text-sm",
        "transition-[filter]",
        variant !== "skelton" &&
          "rounded-md bg-neutral-800 border border-transparent shadow-md",
        "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-900",
        "disabled:opacity-50 disabled:shadow-none",
        !disabled &&
          "hover:brightness-90 active:brightness-90 focus:brightness-90",
        variantClasses,
        sizeClasses,
        className,
      )}
      disabled={disabled}
      {...props}
    >
      {children}
    </button>
  );
};
