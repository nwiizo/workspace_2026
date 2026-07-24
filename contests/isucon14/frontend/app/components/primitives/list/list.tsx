import { ComponentProps, FC, PropsWithChildren } from "react";

export const List: FC<PropsWithChildren<ComponentProps<"ul">>> = ({
  children,
  className,
  ...props
}) => {
  return (
    <ul {...props} className={className}>
      {children}
    </ul>
  );
};
