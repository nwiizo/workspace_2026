import type { MetaFunction } from "@remix-run/node";
import { Link, useNavigate } from "@remix-run/react";
import { useState } from "react";
import { Button } from "~/components/primitives/button/button";
import { TextInput } from "~/components/primitives/form/text-input";
import { FormFrame } from "~/components/primitives/frame/form-frame";
import { getUsers } from "~/utils/get-initial-data";

export const meta: MetaFunction = () => {
  return [
    { title: "Login | ISURIDE" },
    { name: "description", content: "ユーザーログイン" },
  ];
};

export default function ClientLogin() {
  const [sessionToken, setSessionToken] = useState<string>();
  const navigate = useNavigate();
  const presetUsers = getUsers();

  const handleOnClick = () => {
    document.cookie = `app_session=${sessionToken}; path=/`;
    navigate("/client");
  };

  return (
    <FormFrame>
      <h1 className="text-2xl font-semibold mb-6">ユーザーログイン</h1>
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
            {presetUsers.map((preset) => (
              <li key={preset.id}>
                <button
                  className="text-blue-600 hover:underline"
                  onClick={() => setSessionToken(preset.token)}
                >
                  {preset.username}
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
        <p className="text-center mt-2">
          <Link to="/client/register" className="text-blue-600 hover:underline">
            新規登録
          </Link>
        </p>
      </div>
    </FormFrame>
  );
}
