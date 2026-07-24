import { ComponentPropsWithoutRef, FC } from "react";
import { Text } from "~/components/primitives/text/text";

type DateTextProps = Omit<ComponentPropsWithoutRef<typeof Text>, "children"> & {
  value: number;
};

const formatter = new Intl.DateTimeFormat("ja-JP", {
  dateStyle: "medium",
  timeZone: "Asia/Tokyo",
});

export const DateText: FC<DateTextProps> = ({ value, ...rest }) => {
  return <Text {...rest}>{formatter.format(value)}</Text>;
};
