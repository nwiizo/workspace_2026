import { PropsWithChildren } from "react";
import { twMerge } from "tailwind-merge";

export function SmartPhone({ children }: PropsWithChildren) {
  return (
    <div
      className={twMerge(
        "w-[420px] h-[798px] lg:w-[480px] lg:h-[900px] bg-black p-[20px] rounded-[60px] relative",
        "before:block before:bg-black before:absolute before:left-1/2 before:-translate-x-1/2",
        "before:w-[200px] before:h-[30px] before:rounded-bl-3xl before:rounded-br-3xl shrink-0",
      )}
    >
      <div className="h-full flex flex-col overflow-hidden rounded-[40px] before:block before:h-[30px] before:bg-sky-700">
        {children}
      </div>
    </div>
  );
}
