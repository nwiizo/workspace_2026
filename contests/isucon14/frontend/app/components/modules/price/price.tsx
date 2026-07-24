import { ComponentProps, FC } from "react";
import { twMerge } from "tailwind-merge";
import { MoneyIcon } from "~/components/icon/money";
import { Text } from "~/components/primitives/text/text";
import { PriceText } from "../price-text/price-text";

type PriceTextProps = {
  value: number;
  discount?: number;
  pre?: string;
} & ComponentProps<"div">;

export const Price: FC<PriceTextProps> = ({
  value,
  pre,
  discount,
  className,
  ...props
}) => {
  return (
    <div
      className={twMerge(
        "flex flex-col min-[440px]:flex-row space-x-1 items-center",
        className,
      )}
      {...props}
    >
      <div className="flex items-center space-x-1">
        <MoneyIcon width={30} height={30} />
        {pre && (
          <Text tagName="span" className="pr-2">
            {pre}:
          </Text>
        )}
        <PriceText value={value} />
      </div>
      {!!discount && (
        <Text
          tagName="span"
          className="flex flex-row space-x-1 items-center"
          size="sm"
        >
          （
          <Text tagName="span" className="pr-2">
            割引額:
          </Text>
          <PriceText value={discount} />）
        </Text>
      )}
    </div>
  );
};
