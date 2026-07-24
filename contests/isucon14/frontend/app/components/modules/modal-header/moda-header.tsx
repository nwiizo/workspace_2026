import type { ComponentProps, FC, PropsWithChildren } from "react";
import { twMerge } from "tailwind-merge";
import { Text } from "~/components/primitives/text/text";

export const ModalHeader: FC<
  { title: string; subTitle: string } & PropsWithChildren<ComponentProps<"div">>
> = ({ title, subTitle, className, children, ...props }) => {
  return (
    <div
      className={twMerge(
        "flex flex-col items-center justify-center",
        className,
      )}
      {...props}
    >
      {children}
      <Text bold className={twMerge("mb-3", !!children && "mt-8")}>
        {title}
      </Text>
      <Text size="xl" className="mb-8 text-neutral-500 text-center">
        {subTitle}
      </Text>
    </div>
  );
};
