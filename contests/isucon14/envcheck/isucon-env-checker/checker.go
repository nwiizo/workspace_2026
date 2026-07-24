package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strconv"
	"strings"

	"github.com/aws/aws-sdk-go-v2/feature/ec2/imds"
	"github.com/aws/aws-sdk-go-v2/service/ec2"
	"github.com/aws/aws-sdk-go-v2/service/ec2/types"
)

type CheckConfig struct {
	Name string `json:"name"`
	AMI  string `json:"ami_id"`
	AZ   string `json:"az_id"`
}

type checker struct {
	ExpectedAMI          string
	ExpectedAZ           string
	ExpectedInstanceType types.InstanceType
	ExpectedVolumeSize   int

	InstanceIP    string
	InstanceID    string
	InstanceVPCID string

	DescribeInstances         []*ec2.DescribeInstancesOutput
	DescribeVolumes           []*ec2.DescribeVolumesOutput
	DescribeNetworkInterfaces []*ec2.DescribeNetworkInterfacesOutput
	DescribeSecurityGroups    []*ec2.DescribeSecurityGroupsOutput

	DescribeAvailabilityZones *ec2.DescribeAvailabilityZonesOutput

	failures []string

	adminLog    *bytes.Buffer
	adminLogger *log.Logger
}

type Result struct {
	Name         string
	Passed       bool
	IPAddress    string
	Message      string
	AdminMessage string
	RawData      string
}

func Check(ctx context.Context, cfg CheckConfig) (Result, error) {
	buf := new(bytes.Buffer)
	logger := log.New(buf, "", log.LstdFlags)
	c := &checker{
		ExpectedAMI:          cfg.AMI,
		ExpectedAZ:           cfg.AZ,
		ExpectedInstanceType: "c5.large",
		ExpectedVolumeSize:   20,

		adminLog:    buf,
		adminLogger: logger,
	}
	if err := c.loadAWS(ctx); err != nil {
		c.adminLogger.Printf("loading AWS data: %+v", err)
		raw, _ := json.Marshal(c)
		return Result{
			Name:         cfg.Name,
			Passed:       false,
			Message:      "AWS との通信でエラーが発生しました",
			AdminMessage: c.adminLog.String(),
			RawData:      string(raw),
		}, err
	}

	c.checkAll()

	raw, _ := json.Marshal(c)
	return Result{
		Name:         c.name(),
		Passed:       len(c.failures) == 0,
		IPAddress:    c.InstanceIP,
		Message:      c.message(),
		AdminMessage: c.adminLog.String(),
		RawData:      string(raw),
	}, nil
}

func (c *checker) loadAWS(ctx context.Context) error {
	cfg, err := NewAWSSession(ctx)
	if err != nil {
		return fmt.Errorf("creating session: %w", err)
	}
	imdsclient := imds.NewFromConfig(cfg)
	ec2client := ec2.NewFromConfig(cfg)

	c.InstanceIP, err = GetPublicIP(ctx, imdsclient)
	if err != nil {
		return fmt.Errorf("GetPublicIP: %w", err)
	}
	c.InstanceVPCID, err = GetVPC(ctx, imdsclient)
	if err != nil {
		return fmt.Errorf("GetVPC: %w", err)
	}

	c.InstanceID, err = GetInstanceID(ctx, imdsclient)
	if err != nil {
		return fmt.Errorf("GetInstanceID: %w", err)
	}

	c.DescribeInstances, err = DescribeInstances(ctx, ec2client, c.InstanceVPCID)
	if err != nil {
		return fmt.Errorf("DescribeInstances: %w", err)
	}
	c.DescribeVolumes, err = DescribeVolumes(ctx, ec2client, c.DescribeInstances)
	if err != nil {
		return fmt.Errorf("DescribeVolumes: %w", err)
	}
	c.DescribeNetworkInterfaces, err = DescribeNetworkInterfaces(ctx, ec2client, c.InstanceVPCID)
	if err != nil {
		return fmt.Errorf("DescribeNetworkInterfaces: %w", err)
	}
	c.DescribeSecurityGroups, err = DescribeSecurityGroups(ctx, ec2client, c.DescribeInstances)
	if err != nil {
		return fmt.Errorf("DescribeSecurityGroups: %w", err)
	}
	c.DescribeAvailabilityZones, err = ec2client.DescribeAvailabilityZones(ctx, nil)
	if err != nil {
		return fmt.Errorf("DescribeAvailabilityZones: %w", err)
	}
	return nil
}

func (c *checker) addFailure(format string, a ...interface{}) {
	c.failures = append(c.failures, fmt.Sprintf(format, a...))
}

func (c *checker) message() string {
	if len(c.failures) == 0 {
		return "全てのチェックをパスしました"
	}
	return strconv.Itoa(len(c.failures)) + "個の問題があります\n" + strings.Join(c.failures, "\n")
}

func (c *checker) name() string {
	for _, o := range c.DescribeInstances {
		for _, r := range o.Reservations {
			for _, i := range r.Instances {
				id := *i.InstanceId
				if id != c.InstanceID {
					continue
				}
				for _, t := range i.Tags {
					if *t.Key != "Name" {
						continue
					}
					name := *t.Value
					if checkName, ok := checkNameByInstanceName[name]; ok {
						return checkName
					} else {
						return "contest-unknown"
					}
				}
			}
		}
	}
	return "contest-unknown"
}
