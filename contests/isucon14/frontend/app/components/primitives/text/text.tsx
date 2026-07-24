import { ComponentPropsWithoutRef, FC, PropsWithChildren } from "react";
import { twMerge } from "tailwind-merge";

type Size = "2xl" | "xl" | "lg" | "sm" | "xs";

type Variant = "danger" | "normal";

type Tag = "p" | "span" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6";

type TextProps = PropsWithChildren<{
  tagName?: Tag;
  bold?: boolean;
  size?: Size;
  variant?: Variant;
  className?: string;
}> &
  ComponentPropsWithoutRef<Tag>;

const getSizeClass = (size?: Size) => {
  switch (size) {
    case "2xl":
      return "text-2xl";
    case "xl":
      return "text-xl";
    case "lg":
      return "text-lg";
    case "sm":
      return "text-sm";
    case "xs":
      return "text-xs";
    default:
      return "";
  }
};

const getVariantClass = (variant?: Variant) => {
  switch (variant) {
    case "danger":
      return "text-red-500";
    default:
      return "";
  }
};

export const Text: FC<TextProps> = ({
  tagName = "p",
  bold,
  size,
  variant,
  className,
  children,
  ...props
}) => {
  const Tag = tagName;
  return (
    <Tag
      className={twMerge([
        bold && "font-bold",
        getSizeClass(size),
        getVariantClass(variant),
        className,
      ])}
      {...props}
    >
      {children}
    </Tag>
  );
};
