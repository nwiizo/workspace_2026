import { ComponentProps, FC } from "react";
import { twMerge } from "tailwind-merge";
import { Text } from "~/components/primitives/text/text";

type PriceTextProps = {
  value: number;
} & ComponentProps<"span">;

const formatter = new Intl.NumberFormat("ja-JP");

export const PriceText: FC<PriceTextProps> = ({
  value,
  className,
  ...props
}) => {
  return (
    <span
      className={twMerge("flex flex-row space-x-1 items-center", className)}
      {...props}
    >
      <Text tagName="span" size="xl" bold className="font-mono">
        {formatter.format(value)}
      </Text>
      <Text tagName="span" size="sm" bold className="font-mono">
        円
      </Text>
    </span>
  );
};
