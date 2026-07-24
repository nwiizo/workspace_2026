package world

import (
	"fmt"
	"math/rand/v2"
	"slices"
	"strconv"

	"github.com/samber/lo"
)

type ChairModel struct {
	Name  string
	Speed int
}

func (m *ChairModel) GenerateName() string {
	return modelCodes[m.Name] + "-" + strconv.Itoa(rand.IntN(10000))
}

type ChairModels []*ChairModel

func (arr ChairModels) Random() *ChairModel {
	return arr[rand.IntN(len(arr))]
}

var (
	modelNamesBySpeed = map[int][]string{
		2: {
			"リラックスシート NEO",
			"エアシェル ライト",
			"チェアエース S",
			"ベーシックスツール プラス",
			"SitEase",
			"スピンフレーム 01",
			"LiteLine",
			"リラックス座",
			"EasySit",
			"ComfortBasic",
		},
		3: {
			"フォームライン RX",
			"StyleSit",
			"エルゴクレスト II",
			"クエストチェア Lite",
			"AeroSeat",
			"エアフロー EZ",
			"ゲーミングシート NEXUS",
			"シェルシート ハイブリッド",
			"フレックスコンフォート PRO",
			"プレイスタイル Z",
			"ストリームギア S1",
			"リカーブチェア スマート",
			"ErgoFlex",
			"BalancePro",
			"風雅（ふうが）チェア",
		},
		5: {
			"ゼンバランス EX",
			"シャドウバースト M",
			"フューチャーチェア CORE",
			"プレミアムエアチェア ZETA",
			"プロゲーマーエッジ X1",
			"モーションチェア RISE",
			"雅楽座",
			"スリムライン GX",
			"Infinity Seat",
			"LuxeThrone",
			"Titanium Line",
			"ZenComfort",
			"アルティマシート X",
			"インペリアルクラフト LUXE",
			"ステルスシート ROGUE",
		},
		7: {
			"エコシート リジェネレイト",
			"フューチャーステップ VISION",
			"インフィニティ GEAR V",
			"オブシディアン PRIME",
			"ナイトシート ブラックエディション",
			"ShadowEdition",
			"Phoenix Ultra",
			"タイタンフレーム ULTRA",
			"Legacy Chair",
			"ルミナスエアクラウン",
			"ヴァーチェア SUPREME",
			"匠座 PRO LIMITED",
			"匠座（たくみざ）プレミアム",
			"ゼノバース ALPHA",
			"Aurora Glow",
		},
	}
	modelCodes = map[string]string{
		"リラックスシート NEO":      "RS-NEO01",
		"エアシェル ライト":         "AS-LT02",
		"チェアエース S":          "CA-S03",
		"ベーシックスツール プラス":     "BS-PL04",
		"SitEase":           "SE-05",
		"スピンフレーム 01":        "SF-01",
		"LiteLine":          "LL-06",
		"リラックス座":            "RZ-07",
		"EasySit":           "ES-08",
		"ComfortBasic":      "CB-09",
		"フォームライン RX":        "FL-RX10",
		"StyleSit":          "SS-11",
		"エルゴクレスト II":        "EC-II12",
		"クエストチェア Lite":      "QC-L13",
		"AeroSeat":          "AS-14",
		"エアフロー EZ":          "AF-EZ15",
		"ゲーミングシート NEXUS":    "GS-NX16",
		"シェルシート ハイブリッド":     "SS-HY17",
		"フレックスコンフォート PRO":   "FC-P18",
		"プレイスタイル Z":         "PS-Z19",
		"ストリームギア S1":        "SG-S120",
		"リカーブチェア スマート":      "RC-SM21",
		"ErgoFlex":          "EF-22",
		"BalancePro":        "BP-23",
		"風雅（ふうが）チェア":        "FG-C24",
		"ゼンバランス EX":         "ZB-EX25",
		"シャドウバースト M":        "SB-M26",
		"フューチャーチェア CORE":    "FC-CR27",
		"プレミアムエアチェア ZETA":   "PA-Z28",
		"プロゲーマーエッジ X1":      "PE-X129",
		"モーションチェア RISE":     "MC-R30",
		"雅楽座":               "GA-31",
		"スリムライン GX":         "SL-GX32",
		"Infinity Seat":     "IS-33",
		"LuxeThrone":        "LT-34",
		"Titanium Line":     "TL-35",
		"ZenComfort":        "ZC-36",
		"アルティマシート X":        "US-X37",
		"インペリアルクラフト LUXE":   "IC-LX38",
		"ステルスシート ROGUE":     "SS-R39",
		"エコシート リジェネレイト":     "ES-RG40",
		"フューチャーステップ VISION": "FS-V41",
		"インフィニティ GEAR V":    "IG-V42",
		"オブシディアン PRIME":     "OP-43",
		"ナイトシート ブラックエディション": "NS-BE44",
		"ShadowEdition":     "SE-45",
		"Phoenix Ultra":     "PU-46",
		"タイタンフレーム ULTRA":    "TF-U47",
		"Legacy Chair":      "LC-48",
		"ルミナスエアクラウン":        "LA-C49",
		"ヴァーチェア SUPREME":    "VC-S50",
		"匠座 PRO LIMITED":    "TZ-PL51",
		"匠座（たくみざ）プレミアム":     "TZ-P52",
		"ゼノバース ALPHA":       "ZB-A53",
		"Aurora Glow":       "AG-54",
	}
	modelsBySpeed = lo.MapValues(modelNamesBySpeed, func(names []string, speed int) ChairModels {
		return lo.Map(names, func(name string, _ int) *ChairModel {
			return &ChairModel{Name: name, Speed: speed}
		})
	})
	modelSpeeds = lo.Keys(modelNamesBySpeed)
)

func PickModels() map[int]ChairModels {
	result := map[int]ChairModels{}
	for speed, models := range modelsBySpeed {
		result[speed] = lo.Shuffle(slices.Clone(models))[:3]
	}
	return result
}

func PickRandomModel() *ChairModel {
	return modelsBySpeed[modelSpeeds[rand.IntN(len(modelSpeeds))]].Random()
}

func init() {
	for _, names := range modelNamesBySpeed {
		for _, name := range names {
			_, ok := modelCodes[name]
			if !ok {
				panic(fmt.Errorf("コード名がない: %s", name))
			}
		}
	}
}
