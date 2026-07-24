import { FC } from "react";
import { ChairIcon } from "~/components/icon/chair";
import { ChairInformation } from "~/components/modules/chair-information/chair-information";
import { LocationButton } from "~/components/modules/location-button/location-button";
import { ModalHeader } from "~/components/modules/modal-header/moda-header";
import { Price } from "~/components/modules/price/price";
import { Text } from "~/components/primitives/text/text";
import { useClientContext } from "~/contexts/client-context";

export const Enroute: FC = () => {
  const { data } = useClientContext();
  return (
    <div className="w-full h-full flex flex-col items-center justify-center max-w-md mx-auto">
      <ModalHeader
        title="椅子が見つかりました"
        subTitle="あなたのもとに向かっています"
      >
        <ChairIcon
          model={data?.chair?.model ?? ""}
          width={100}
          className="animate-shake"
        />
      </ModalHeader>
      {data?.chair && <ChairInformation className="mb-8" chair={data.chair} />}
      <LocationButton
        label="現在地"
        location={data?.pickup_coordinate}
        className="w-full"
        disabled
      />
      <Text size="xl">↓</Text>
      <LocationButton
        label="目的地"
        location={data?.destination_coordinate}
        className="w-full"
        disabled
      />
      {data?.fare && (
        <Price pre="運賃" value={data.fare} className="mt-8"></Price>
      )}
    </div>
  );
};
