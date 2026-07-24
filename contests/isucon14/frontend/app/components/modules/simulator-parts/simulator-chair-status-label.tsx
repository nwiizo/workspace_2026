import { ComponentProps, FC } from "react";
import { twMerge } from "tailwind-merge";
import colors from "tailwindcss/colors";
import { RideStatus } from "~/api/api-schemas";
import { Text } from "~/components/primitives/text/text";

const StatusList = {
  MATCHING: ["空車", colors.sky["500"]],
  ENROUTE: ["迎車", colors.amber["500"]],
  PICKUP: ["乗車待ち", colors.amber["500"]],
  CARRYING: ["賃走", colors.red["500"]],
  ARRIVED: ["到着", colors.emerald["500"]],
  COMPLETED: ["空車", colors.sky["500"]],
} as const satisfies Record<RideStatus, [string, string]>;

export const SimulatorChairRideStatus: FC<
  ComponentProps<"div"> & {
    currentStatus: RideStatus;
  }
> = ({ currentStatus, className, ...props }) => {
  const [labelName, color] = StatusList[currentStatus];
  return (
    <div
      className={twMerge("font-bold flex items-center space-x-1", className)}
      {...props}
    >
      <div
        role="presentation"
        className="w-3 h-3 rounded-full"
        style={{ backgroundColor: color }}
      />
      <Text size="sm" style={{ color }}>
        {labelName}
      </Text>
    </div>
  );
};
