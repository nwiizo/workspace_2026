import type { MetaFunction } from "@remix-run/node";
import { Link, useNavigate } from "@remix-run/react";
import { useState } from "react";
import { Button } from "~/components/primitives/button/button";
import { TextInput } from "~/components/primitives/form/text-input";
import { FormFrame } from "~/components/primitives/frame/form-frame";
import { getOwners } from "~/utils/get-initial-data";

export const meta: MetaFunction = () => {
  return [
    { title: "Login | Owner | ISURIDE" },
    { name: "description", content: "オーナーログイン" },
  ];
};

export default function OwnerLogin() {
  const [sessionToken, setSessionToken] = useState<string>();
  const navigate = useNavigate();
  const presetOwners = getOwners();

  const handleOnClick = () => {
    document.cookie = `owner_session=${sessionToken}; path=/`;
    navigate("/owner");
  };

  return (
    <FormFrame>
      <h1 className="text-2xl font-semibold mb-6">オーナーログイン</h1>
      <div className="mb-4">
        <TextInput
          id="sessionToken"
          name="sessionToken"
          label="セッショントークン"
          defaultValue={sessionToken}
          onChange={(e) => setSessionToken(e.target.value)}
          value={sessionToken}
        />
        <details className="mt-3">
          <summary className="mb-2 ps-2 cursor-pointer">presetから選択</summary>
          <ul className="list-disc ps-8 space-y-1">
            {presetOwners.map((preset) => (
              <li key={preset.id}>
                <button
                  className="text-blue-600 hover:underline"
                  onClick={() => setSessionToken(preset.token)}
                >
                  {preset.name}
                </button>
              </li>
            ))}
          </ul>
        </details>
      </div>
      <div className="flex flex-col gap-4">
        <Button
          variant="primary"
          className="text-lg"
          onClick={() => void handleOnClick()}
        >
          ログイン
        </Button>
        <p className="text-center">
          <Link to="/owner/register" className="text-blue-600 hover:underline">
            新規登録
          </Link>
        </p>
      </div>
    </FormFrame>
  );
}
