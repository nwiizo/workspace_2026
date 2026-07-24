import type { MetaFunction } from "@remix-run/node";
import { useEffect, useState } from "react";
import { AppGetRidesResponse, fetchAppGetRides } from "~/api/api-components";
import { ChairIcon } from "~/components/icon/chair";
import { DateText } from "~/components/modules/date-text/date-text";
import { Price } from "~/components/modules/price/price";
import { List } from "~/components/primitives/list/list";
import { ListItem } from "~/components/primitives/list/list-item";
import { Rating } from "~/components/primitives/rating/rating";
import { Text } from "~/components/primitives/text/text";

export const meta: MetaFunction = () => {
  return [
    { title: "History | ISURIDE" },
    { name: "description", content: "椅子の配車履歴" },
  ];
};

type Ride = AppGetRidesResponse["rides"][number];

export default function Index() {
  const [rides, setRides] = useState<Ride[]>([]);

  useEffect(() => {
    void (async () => {
      try {
        const res = await fetchAppGetRides({});
        setRides(res.rides);
      } catch (error) {
        console.error(error);
      }
    })();
  }, []);

  return (
    <section className="mx-8 flex-1">
      <h2 className="text-xl my-6">履歴</h2>
      <List className="border-t">
        {rides.length === 0 && (
          <ListItem>
            <Text className="py-10 text-neutral-500">
              椅子の乗車履歴はありません
            </Text>
          </ListItem>
        )}
        {rides.map(
          ({
            id,
            fare,
            completed_at,
            pickup_coordinate,
            destination_coordinate,
            chair,
            evaluation,
          }) => (
            <ListItem key={id} className="py-4">
              <div className="flex justify-between mb-3">
                <div className="flex-grow space-y-1.5">
                  <DateText
                    value={completed_at}
                    tagName="span"
                    className="font-bold"
                    size="xl"
                  />
                  <Text size="sm">
                    {`[${pickup_coordinate.latitude}, ${pickup_coordinate.longitude}] から [${destination_coordinate.latitude}, ${destination_coordinate.longitude}] への移動`}
                  </Text>
                </div>
                <Price value={fare} />
              </div>
              <div className="flex space-x-2 items-center bg-neutral-100 py-2 px-4 rounded-md justify-between w-full">
                <div className="flex space-x-4 items-center">
                  <ChairIcon
                    model={chair.model}
                    width={20}
                    height={20}
                    className="shrink-0"
                  />
                  <div className="flex items-baseline flex-col">
                    <Text tagName="span">{chair.name}</Text>
                    <Text
                      size="xs"
                      tagName="span"
                      className={"text-neutral-500"}
                    >
                      {chair.model}
                    </Text>
                  </div>
                </div>
                <Rating rating={evaluation} size={20} />
              </div>
            </ListItem>
          ),
        )}
      </List>
    </section>
  );
}
