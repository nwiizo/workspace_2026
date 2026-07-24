package benchrun

import (
	"crypto/md5"
	_ "embed"
	"encoding/hex"
	"encoding/json"
	"io"
	"slices"

	"github.com/samber/lo"
)

//go:embed frontend_hashes.json
var frontendHashes []byte

//go:embed frontend_files.json
var frontendFiles []byte

type FrontendPathScenario int

var (
	FrontendHashesMap         = make(map[string]string)
	FrontendPathScenarioFiles = make(map[FrontendPathScenario][]string)
)

const (
	FRONTEND_PATH_SCENARIO_CLIENT_REGISTER_1 FrontendPathScenario = iota
	FRONTEND_PATH_SCENARIO_CLIENT_REGISTER_2
	FRONTEND_PATH_SCENARIO_CLIENT_REGISTER_3
	FRONTEND_PATH_SCENARIO_CLIENT_EVALUATION
	FRONTEND_PATH_SCENARIO_CLIENT_CHECK_HISTORY_1
	FRONTEND_PATH_SCENARIO_CLIENT_CHECK_HISTORY_2

	FRONTEND_PATH_SCENARIO_OWNER_REGISTER_1
	FRONTEND_PATH_SCENARIO_OWNER_REGISTER_2
	FRONTEND_PATH_SCENARIO_OWNER_CHAIRS
	FRONTEND_PATH_SCENARIO_OWNER_SALES
)

var FRONTEND_PATH_SCENARIOS = map[FrontendPathScenario][]string{
	// /clientにハードナビゲーションしたとき
	FRONTEND_PATH_SCENARIO_CLIENT_REGISTER_1: {"/client"},
	// /clientにハードナビゲーションして、
	// /client/historyにソフトナビゲーションしたとき
	FRONTEND_PATH_SCENARIO_CLIENT_REGISTER_2: {"/client", "/client/register"},
	FRONTEND_PATH_SCENARIO_CLIENT_REGISTER_3: {"/client", "/client/register", "/client/register-payment"},
	FRONTEND_PATH_SCENARIO_CLIENT_EVALUATION: {"/client"},
	FRONTEND_PATH_SCENARIO_CLIENT_CHECK_HISTORY_1: {"/client"},
	FRONTEND_PATH_SCENARIO_CLIENT_CHECK_HISTORY_2: {"/client", "/client/history"},

	FRONTEND_PATH_SCENARIO_OWNER_REGISTER_1: {"/owner"},
	FRONTEND_PATH_SCENARIO_OWNER_REGISTER_2: {"/owner", "/owner/register"},
	FRONTEND_PATH_SCENARIO_OWNER_CHAIRS:     {"/owner"},
	FRONTEND_PATH_SCENARIO_OWNER_SALES:      {"/owner", "/owner/sales"},
}

func init() {
	err := json.Unmarshal(frontendHashes, &FrontendHashesMap)
	if err != nil {
		panic(err)
	}

	frontendFilesMap := make(map[string][]string)
	err = json.Unmarshal(frontendFiles, &frontendFilesMap)
	if err != nil {
		panic(err)
	}

	for scenario, ps := range FRONTEND_PATH_SCENARIOS {
		paths := slices.Clone(ps)
		slices.Reverse(paths)
		files := frontendFilesMap[paths[0]]
		for _, path := range paths[1:] {
			fetchedFiles := frontendFilesMap[path]
			if len(fetchedFiles) > 0 {
				files = lo.Filter(files, func(item string, index int) bool {
					return !slices.Contains(fetchedFiles, item)
				})
			}
		}
		FrontendPathScenarioFiles[scenario] = files
	}
}

func GetHashFromStream(r io.Reader) (string, error) {
	h := md5.New()
	if _, err := io.Copy(h, r); err != nil {
		return "", err
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}
