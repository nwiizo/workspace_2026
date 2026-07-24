import type { MetaFunction } from "@remix-run/node";
import { ChairIcon } from "~/components/icon/chair";
import { Text } from "~/components/primitives/text/text";
import { useOwnerContext } from "~/contexts/owner-context";

export const meta: MetaFunction = () => {
  return [
    { title: "椅子一覧 | Owner | ISURIDE" },
    { name: "description", content: "isucon14" },
  ];
};

const formatDateTime = (timestamp: number) => {
  const d = new Date(timestamp);
  return `${d.getFullYear()}/${d.getMonth() + 1}/${d.getDate()} ${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}`;
};

export default function Index() {
  const { chairs } = useOwnerContext();

  return (
    <div className="min-w-[1050px] w-full">
      {chairs?.length ? (
        <table className="text-sm w-full">
          <thead className="bg-gray-50 border-b">
            <tr className="text-gray-500">
              <th className="px-4 py-3 text-left">ID</th>
              <th className="px-4 py-3 text-left">名前</th>
              <th className="px-4 py-3 text-left">モデル</th>
              <th className="px-4 py-3 text-left">状態</th>
              <th className="px-4 py-3 text-left">総走行距離</th>
              <th className="px-4 py-3 text-left">登録日</th>
            </tr>
          </thead>
          <tbody>
            {chairs.map((chair) => (
              <tr
                key={chair.id}
                className="border-b hover:bg-gray-50 transition"
              >
                <td className="p-4 font-mono">{chair.id}</td>
                <td className="p-4 max-w-48 truncate" title={chair.name}>
                  {chair.name}
                </td>
                <td className="p-4 max-w-64" title={chair.model}>
                  <div className="flex">
                    <ChairIcon
                      model={chair.model}
                      className="shrink-0 size-6 me-2"
                    />
                    <span className="flex-1 truncate">{chair.model}</span>
                  </div>
                </td>
                <td className="p-4">
                  <div className="">
                    <span
                      className={`before:content-['●'] before:mr-2 ${chair.active ? "before:text-emerald-600" : "before:text-red-600"}`}
                    >
                      {chair.active ? "稼働中" : "停止中"}
                    </span>
                  </div>
                </td>
                <td className="p-4 text-right font-mono">
                  {chair.total_distance}
                </td>
                <td className="p-4 font-mono">
                  {formatDateTime(chair.registered_at)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <Text>登録されている椅子がありません</Text>
      )}
    </div>
  );
}
