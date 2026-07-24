package cmd

import (
	"fmt"
	"math/rand/v2"
	"strconv"
	"time"

	"github.com/go-sql-driver/mysql"
	"github.com/guregu/null/v5"
	"github.com/isucon/isucon14/bench/benchmarker/world"
	"github.com/isucon/isucon14/bench/internal/random"
	"github.com/jmoiron/sqlx"
	"github.com/oklog/ulid/v2"
	"github.com/spf13/cobra"
)

var (
	dbUser     string
	dbPassword string
	dbAddr     string
	dbName     string
)

var (
	baseUserNum       = 300
	inviteProbability = 0.5
	ownerNum          = 5
	chairNumPerOwner  = 100
	rideNum           = 750
	averageDistance   = 25
)

var generateInitDataCmd = &cobra.Command{
	Use:   "generate-init-data",
	Short: "初期データをDBに生成する",
	Long: `
初期データ生成し、指定されたDBに挿入します。
実行時にride_statuses, rides, chair_locations, chairs, owners, payment_tokens, coupons, usersテーブルの行は全て削除されます。
実行後にride_statuses, rides, chair_locations, chairs, owners, payment_tokens, coupons, usersテーブルをmysqldumpしてください

$ mysqldump --skip-create-options --skip-add-drop-table --disable-keys --no-create-info --no-tablespaces -h 127.0.0.1 -u isucon -p --databases isuride -n --ignore-table=isuride.settings --ignore-table=isuride.chair_models | gzip > ./3-initial-data.sql.gz
`,
	RunE: func(cmd *cobra.Command, args []string) error {
		rand := random.NewLockedRand(rand.NewPCG(0, 0))

		dbConfig := mysql.NewConfig()
		dbConfig.User = dbUser
		dbConfig.Passwd = dbPassword
		dbConfig.Addr = dbAddr
		dbConfig.Net = "tcp"
		dbConfig.DBName = dbName
		dbConfig.ParseTime = true

		baseTime := time.Date(2024, 11, 25, 0, 0, 0, 0, time.FixedZone("Asia/Tokyo", 8*60*60))
		region := world.NewRegion("チェアタウン", 0, 0, 100, 100)

		db, err := sqlx.Connect("mysql", dbConfig.FormatDSN())
		if err != nil {
			return fmt.Errorf("failed to connect db: %w", err)
		}

		type Coupon struct {
			UserID    string      `db:"user_id"`
			Code      string      `db:"code"`
			Discount  int         `db:"discount"`
			CreatedAt time.Time   `db:"created_at"`
			UsedBy    null.String `db:"used_by"`
		}
		type User struct {
			ID             string    `db:"id"`
			UserName       string    `db:"username"`
			FirstName      string    `db:"firstname"`
			LastName       string    `db:"lastname"`
			DateOfBirth    string    `db:"date_of_birth"`
			AccessToken    string    `db:"access_token"`
			InvitationCode string    `db:"invitation_code"`
			CreatedAt      time.Time `db:"created_at"`
			UpdatedAt      time.Time `db:"updated_at"`

			coupons []*Coupon `db:"-"`
		}
		type PaymentToken struct {
			UserID    string    `db:"user_id"`
			Token     string    `db:"token"`
			CreatedAt time.Time `db:"created_at"`
		}
		type Owner struct {
			ID                 string    `db:"id"`
			Name               string    `db:"name"`
			AccessToken        string    `db:"access_token"`
			ChairRegisterToken string    `db:"chair_register_token"`
			CreatedAt          time.Time `db:"created_at"`
			UpdatedAt          time.Time `db:"updated_at"`
		}
		type ChairLocation struct {
			ID        string    `db:"id"`
			ChairID   string    `db:"chair_id"`
			Latitude  int       `db:"latitude"`
			Longitude int       `db:"longitude"`
			CreatedAt time.Time `db:"created_at"`
		}
		type Chair struct {
			ID          string    `db:"id"`
			OwnerID     string    `db:"owner_id"`
			Name        string    `db:"name"`
			Model       string    `db:"model"`
			IsActive    bool      `db:"is_active"`
			AccessToken string    `db:"access_token"`
			CreatedAt   time.Time `db:"created_at"`
			UpdatedAt   time.Time `db:"updated_at"`

			modelData            *world.ChairModel `db:"-"`
			initialChairLocation world.Coordinate  `db:"-"`
			locations            []*ChairLocation  `db:"-"`
		}
		type Ride struct {
			ID                   string    `db:"id"`
			UserID               string    `db:"user_id"`
			ChairID              string    `db:"chair_id"`
			PickupLatitude       int       `db:"pickup_latitude"`
			PickupLongitude      int       `db:"pickup_longitude"`
			DestinationLatitude  int       `db:"destination_latitude"`
			DestinationLongitude int       `db:"destination_longitude"`
			Evaluation           int       `db:"evaluation"`
			CreatedAt            time.Time `db:"created_at"`
			UpdatedAt            time.Time `db:"updated_at"`
		}
		type RideStatus struct {
			ID          string    `db:"id"`
			RideID      string    `db:"ride_id"`
			Status      string    `db:"status"`
			CreatedAt   time.Time `db:"created_at"`
			AppSentAt   time.Time `db:"app_sent_at"`
			ChairSentAt time.Time `db:"chair_sent_at"`
		}

		var (
			owners       []*Owner
			chairs       []*Chair
			users        []*User
			rides        []*Ride
			rideStatuses []*RideStatus
		)

		for range ownerNum {
			registeredAt := baseTime
			owner := &Owner{
				ID:                 makeULIDFromTime(registeredAt),
				Name:               random.GenerateOwnerName(),
				AccessToken:        random.GenerateHexString(64),
				ChairRegisterToken: random.GenerateHexString(64),
				CreatedAt:          registeredAt,
				UpdatedAt:          registeredAt,
			}
			owners = append(owners, owner)

			for range chairNumPerOwner {
				createdAt := registeredAt.Add(time.Duration(rand.IntN(82800)) * time.Second)
				model := world.PickRandomModel()
				chair := &Chair{
					ID:                   makeULIDFromTime(createdAt),
					OwnerID:              owner.ID,
					Name:                 model.GenerateName(),
					Model:                model.Name,
					IsActive:             false,
					AccessToken:          random.GenerateHexString(64),
					CreatedAt:            createdAt,
					UpdatedAt:            createdAt,
					modelData:            model,
					initialChairLocation: world.RandomCoordinateOnRegion(region),
				}
				chairs = append(chairs, chair)
			}
		}

		for range baseUserNum {
			registeredAt := baseTime.AddDate(0, 0, 1).Add(time.Duration(rand.IntN(82800)) * time.Second)
			user := &User{
				ID:             makeULIDFromTime(registeredAt),
				UserName:       random.GenerateUserName(),
				FirstName:      random.GenerateFirstName(),
				LastName:       random.GenerateLastName(),
				DateOfBirth:    random.GenerateDateOfBirth(),
				AccessToken:    random.GenerateHexString(64),
				InvitationCode: random.GenerateHexString(30),
				CreatedAt:      registeredAt,
				UpdatedAt:      registeredAt,
			}
			users = append(users, user)
			for i := range 3 {
				if rand.Float64() < inviteProbability {
					registeredAt := user.CreatedAt.Add(time.Duration(i) * time.Hour)
					invitee := &User{
						ID:             makeULIDFromTime(registeredAt),
						UserName:       random.GenerateUserName(),
						FirstName:      random.GenerateFirstName(),
						LastName:       random.GenerateLastName(),
						DateOfBirth:    random.GenerateDateOfBirth(),
						AccessToken:    random.GenerateHexString(64),
						InvitationCode: random.GenerateHexString(30),
						CreatedAt:      registeredAt,
						UpdatedAt:      registeredAt,
					}
					invitee.coupons = append(invitee.coupons, &Coupon{
						UserID:    invitee.ID,
						Code:      "INV_" + user.InvitationCode,
						Discount:  1500,
						CreatedAt: registeredAt,
					})
					user.coupons = append(user.coupons, &Coupon{
						UserID:    user.ID,
						Code:      "RWD_" + user.InvitationCode + "_" + strconv.FormatInt(registeredAt.UnixMilli(), 10),
						Discount:  1000,
						CreatedAt: registeredAt,
					})
					users = append(users, invitee)
				} else {
					break
				}
			}
		}

		for range rideNum {
			pickup := world.RandomCoordinateOnRegionWithRand(region, rand)
			dest := world.RandomCoordinateAwayFromHereWithRand(pickup, max(int(float64(averageDistance)+10*rand.NormFloat64()), 5), rand)

			chair := chairs[rand.IntN(len(chairs))]
			user := users[rand.IntN(len(users))]

			createdAt := user.CreatedAt.Add(3 * time.Minute)
			start := chair.initialChairLocation
			if len(chair.locations) > 0 {
				lastLocation := chair.locations[len(chair.locations)-1]
				createdAt = lastLocation.CreatedAt.Add(3 * time.Minute)
				start = world.C(lastLocation.Latitude, lastLocation.Longitude)
			} else {
				chair.locations = append(chair.locations, &ChairLocation{
					ID:        makeULIDFromTime(createdAt),
					ChairID:   chair.ID,
					Latitude:  start.X,
					Longitude: start.Y,
					CreatedAt: createdAt,
				})
			}

			ride := &Ride{
				ID:                   makeULIDFromTime(createdAt),
				UserID:               user.ID,
				ChairID:              chair.ID,
				PickupLatitude:       pickup.X,
				PickupLongitude:      pickup.Y,
				DestinationLatitude:  dest.X,
				DestinationLongitude: dest.Y,
				Evaluation:           rand.IntN(4) + 2,
				CreatedAt:            createdAt,
			}
			for _, coupon := range user.coupons {
				if !coupon.UsedBy.Valid {
					coupon.UsedBy = null.StringFrom(ride.ID)
					break
				}
			}

			current := start
			target := pickup
			i := 1
			rideStatuses = append(rideStatuses, &RideStatus{
				ID:          makeULIDFromTime(createdAt),
				RideID:      ride.ID,
				Status:      "MATCHING",
				CreatedAt:   createdAt,
				AppSentAt:   createdAt.Add(1 * time.Second),
				ChairSentAt: createdAt.Add(1 * time.Second),
			})
			matchedAt := createdAt.Add(3 * time.Second)
			rideStatuses = append(rideStatuses, &RideStatus{
				ID:          makeULIDFromTime(matchedAt),
				RideID:      ride.ID,
				Status:      "ENROUTE",
				CreatedAt:   matchedAt,
				AppSentAt:   matchedAt.Add(1 * time.Second),
				ChairSentAt: matchedAt.Add(1 * time.Second),
			})
			for {
				next := current.MoveToward(target, chair.modelData.Speed, rand)
				nextTime := matchedAt.Add(time.Duration(i) * time.Minute)
				chair.locations = append(chair.locations, &ChairLocation{
					ID:        makeULIDFromTime(nextTime),
					ChairID:   chair.ID,
					Latitude:  next.X,
					Longitude: next.Y,
					CreatedAt: nextTime,
				})

				if next.Equals(pickup) {
					target = dest
					rideStatuses = append(rideStatuses, &RideStatus{
						ID:          makeULIDFromTime(nextTime),
						RideID:      ride.ID,
						Status:      "PICKUP",
						CreatedAt:   nextTime,
						AppSentAt:   nextTime.Add(1 * time.Second),
						ChairSentAt: nextTime.Add(1 * time.Second),
					})
					i++
					nextTime = matchedAt.Add(time.Duration(i) * time.Minute)
					rideStatuses = append(rideStatuses, &RideStatus{
						ID:          makeULIDFromTime(nextTime),
						RideID:      ride.ID,
						Status:      "CARRYING",
						CreatedAt:   nextTime,
						AppSentAt:   nextTime.Add(1 * time.Second),
						ChairSentAt: nextTime.Add(1 * time.Second),
					})
				} else if next.Equals(dest) {
					rideStatuses = append(rideStatuses, &RideStatus{
						ID:          makeULIDFromTime(nextTime),
						RideID:      ride.ID,
						Status:      "ARRIVED",
						CreatedAt:   nextTime,
						AppSentAt:   nextTime.Add(1 * time.Second),
						ChairSentAt: nextTime.Add(1 * time.Second),
					})
					i++
					nextTime = matchedAt.Add(time.Duration(i) * time.Minute)
					rideStatuses = append(rideStatuses, &RideStatus{
						ID:          makeULIDFromTime(nextTime),
						RideID:      ride.ID,
						Status:      "COMPLETED",
						CreatedAt:   nextTime,
						AppSentAt:   nextTime.Add(1 * time.Second),
						ChairSentAt: nextTime.Add(1 * time.Second),
					})
					break
				}

				current = next
				i++
			}
			ride.UpdatedAt = matchedAt.Add(time.Duration(i) * time.Minute)
			rides = append(rides, ride)
		}

		// TRUNCATE TABLES
		for _, table := range []string{"ride_statuses", "rides", "chair_locations", "chairs", "owners", "payment_tokens", "coupons", "users"} {
			_, err := db.ExecContext(cmd.Context(), `TRUNCATE TABLE `+table)
			if err != nil {
				return err
			}
		}

		// Ownerデータの挿入
		_, err = db.NamedExecContext(cmd.Context(), `INSERT INTO owners (id, name, access_token, chair_register_token, created_at, updated_at) VALUES (:id, :name, :access_token, :chair_register_token, :created_at, :updated_at)`, owners)
		if err != nil {
			return err
		}

		// Chairデータの挿入
		_, err = db.NamedExecContext(cmd.Context(), `INSERT INTO chairs (id, owner_id, name, model, is_active, access_token, created_at, updated_at) VALUES (:id, :owner_id, :name, :model, :is_active, :access_token, :created_at, :updated_at)`, chairs)
		if err != nil {
			return err
		}

		// ChairLocationデータの挿入
		for _, chair := range chairs {
			if len(chair.locations) == 0 {
				continue
			}
			_, err = db.NamedExecContext(cmd.Context(), `INSERT INTO chair_locations (id, chair_id, latitude, longitude, created_at) VALUES (:id, :chair_id, :latitude, :longitude, :created_at)`, chair.locations)
			if err != nil {
				return err
			}
		}

		// Userデータの挿入
		_, err = db.NamedExecContext(cmd.Context(), `INSERT INTO users (id, username, firstname, lastname, date_of_birth, access_token, invitation_code, created_at, updated_at) VALUES (:id, :username, :firstname, :lastname, :date_of_birth, :access_token, :invitation_code, :created_at, :updated_at)`, users)
		if err != nil {
			return err
		}

		// Couponデータの挿入
		for _, user := range users {
			if len(user.coupons) == 0 {
				continue
			}
			_, err = db.NamedExecContext(cmd.Context(), `INSERT INTO coupons (user_id, code, discount, created_at, used_by) VALUES (:user_id, :code, :discount, :created_at, :used_by)`, user.coupons)
			if err != nil {
				return err
			}
		}

		// PaymentTokenデータを挿入
		for _, user := range users {
			_, err = db.NamedExecContext(cmd.Context(), `INSERT INTO payment_tokens (user_id, token, created_at) VALUES (:user_id, :token, :created_at)`, &PaymentToken{
				UserID:    user.ID,
				Token:     random.GeneratePaymentToken(),
				CreatedAt: user.CreatedAt,
			})
			if err != nil {
				return err
			}
		}

		// Rideデータを挿入
		_, err = db.NamedExecContext(cmd.Context(), `INSERT INTO rides (id, user_id, chair_id, pickup_latitude, pickup_longitude, destination_latitude, destination_longitude, evaluation, created_at, updated_at) VALUES (:id, :user_id, :chair_id, :pickup_latitude, :pickup_longitude, :destination_latitude, :destination_longitude, :evaluation, :created_at, :updated_at)`, rides)
		if err != nil {
			return err
		}

		// RideStatusデータを挿入
		_, err = db.NamedExecContext(cmd.Context(), `INSERT INTO ride_statuses (id, ride_id, status, created_at, app_sent_at, chair_sent_at) VALUES (:id, :ride_id, :status, :created_at, :app_sent_at, :chair_sent_at)`, rideStatuses)
		if err != nil {
			return err
		}

		return nil
	},
}

func init() {
	generateInitDataCmd.Flags().StringVar(&dbUser, "user", "isucon", "db user")
	generateInitDataCmd.Flags().StringVar(&dbPassword, "password", "isucon", "db password")
	generateInitDataCmd.Flags().StringVar(&dbAddr, "addr", "127.0.0.1:3306", "db addr (host:port)")
	generateInitDataCmd.Flags().StringVar(&dbName, "database", "isuride", "db name")
	rootCmd.AddCommand(generateInitDataCmd)
}

func makeULIDFromTime(t time.Time) string {
	return ulid.MustNew(ulid.Timestamp(t), ulid.DefaultEntropy()).String()
}
