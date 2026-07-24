import { FC, useCallback, useMemo, useState } from "react";
import { CopyIcon } from "~/components/icon/copy";
import { Button } from "~/components/primitives/button/button";
import { Text } from "~/components/primitives/text/text";
import { CampaignData } from "~/types";
import { getCampaignData, saveCampaignData } from "~/utils/storage";

export const CampaignBanner: FC = () => {
  const [campaign, setCampaign] = useState<CampaignData | null>(() =>
    getCampaignData(),
  );

  const isShow = useMemo(() => {
    return !!(
      campaign &&
      !campaign.used &&
      Date.now() - new Date(campaign.registedAt).getTime() < 60 * 60 * 1000
    );
  }, [campaign]);

  const onClose = useCallback(() => {
    if (!campaign) return;
    const updatedCampaign = { ...campaign, used: true };
    saveCampaignData(updatedCampaign);
    setCampaign(updatedCampaign);
  }, [campaign]);

  const onClick = useCallback(() => {
    if (!campaign) return;
    const handleCopyCode = async () => {
      try {
        await navigator.clipboard.writeText(campaign.invitationCode);
        alert("招待コードがコピーされました！");
      } catch (error) {
        alert(
          `招待コード: ${campaign.invitationCode}\nコピーしてお使いください`,
        );
      }
    };
    void handleCopyCode();
  }, [campaign]);

  return isShow ? (
    <div className="sticky w-full md:max-w-screen-md px-4 py-3.5 md:pl-8 flex flex-row items-center justify-between bg-gradient-to-br from-sky-700 from-20% to-cyan-300">
      <div className="flex items-center space-x-4">
        <Text className="text-white text-sm">
          今なら友達を招待すると
          <br className="block md:hidden" />
          <span className="font-mono">1000円OFF</span>
        </Text>
        <Button
          type="button"
          className="items-center inline-flex rounded-full bg-cyan-900 text-white px-4 py-2 bg-opacity-50 text-xs shrink-0 hover:bg-opacity-30"
          onClick={onClick}
        >
          <CopyIcon className="mr-1 size-5" />
          招待コード
        </Button>
      </div>
      <Button
        className="-m-3 p-3 hover:opacity-80 rounded-full"
        variant="skelton"
        onClick={onClose}
      >
        <span className="sr-only">閉じる</span>
        <svg
          className="size-6 text-white"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
          data-slot="icon"
        >
          <path d="M6.28 5.22a.75.75 0 0 0-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 1 0 1.06 1.06L10 11.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L11.06 10l3.72-3.72a.75.75 0 0 0-1.06-1.06L10 8.94 6.28 5.22Z" />
        </svg>
      </Button>
    </div>
  ) : null;
};
