package random

import (
	_ "embed"
	"sync/atomic"
	"time"

	"github.com/brianvoe/gofakeit/v7"
	"github.com/mattn/go-gimei"
	"github.com/samber/lo"
)

var (
	dateStart = time.Date(1954, 1, 1, 0, 0, 0, 0, time.UTC)   // 70歳ぐらい
	dateEnd   = time.Date(2006, 12, 31, 0, 0, 0, 0, time.UTC) // 18歳ぐらい
)

var (
	ownerNamesIdx atomic.Int32
	ownerNames    = []string{
		"ReSit",
		"ChairCycle",
		"Vintage Vibe Seats",
		"Sit Second Life",
		"EcoChairs",
		"Seat Revival",
		"Chair Charm",
		"Timeless Seats",
		"ReSit & Renew",
		"PreLoved Chairs",
		"SwiftCab",
		"CityRide",
		"Skyline Taxis",
		"Urban Wheels",
		"GoEase",
		"NextStep Cab",
		"Orbit Taxi",
		"ZenDrive",
		"PulseRide",
		"LuxeLine Taxi",
		"迅行タクシー",
		"街のりタクシー",
		"和やか交通",
		"晴れ道タクシー",
		"快旅タクシー",
		"つばさ交通",
		"ゆったり号",
		"瞬道タクシー",
		"おもてなしタクシー",
		"ひまわり交通",
		"椅子再生工房",
		"ふたたび椅子堂",
		"椅子のめぐりや",
		"座り心地再生所",
		"椅子と暮らし舎",
		"再座堂",
		"椅子やいちば",
		"椅子のふるさと",
		"レトロ椅子館",
		"いすの道",
		"匠椅子製作所",
		"座心工房",
		"木座デザイン",
		"座和インダストリー",
		"椅子工藝舎",
		"快座製作所",
		"暮らしの椅子研究所",
		"つくる椅子株式会社",
		"風座プロダクツ",
		"ZenSeat",
		"CraftChair Co., Ltd.",
		"SitWell Designs",
		"ComfortWorks",
		"Artisan Seats",
	}
	initOwnerNames = []string{
		"Seat Revival",
		"快座製作所",
		"匠椅子製作所",
		"つくる椅子株式会社",
		"NextStep Cab",
	}
)

func init() {
	// 内部データをロードさせておく
	_ = gimei.NewName()
	ownerNames = lo.Shuffle(lo.Filter(ownerNames, func(name string, _ int) bool { return !lo.Contains(initOwnerNames, name) }))
}

func GenerateOwnerName() string {
	return ownerNames[int(ownerNamesIdx.Add(1))%len(ownerNames)]
}
func GenerateLastName() string     { return gimei.NewName().Last.Kanji() }
func GenerateFirstName() string    { return gimei.NewName().First.Kanji() }
func GenerateUserName() string     { return gofakeit.Username() }
func GenerateDateOfBirth() string  { return gofakeit.DateRange(dateStart, dateEnd).Format("2006-01-02") }
func GeneratePaymentToken() string { return gofakeit.LetterN(100) }
func GenerateHexString(n int) string {
	return lo.RandomString(n, []rune("0123456789abcdef"))
}
