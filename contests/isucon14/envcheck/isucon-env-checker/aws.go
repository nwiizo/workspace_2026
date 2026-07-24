package main

import (
	"context"
	"fmt"
	"io"
	"net"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/feature/ec2/imds"
	"github.com/aws/aws-sdk-go-v2/service/ec2"
	"github.com/aws/aws-sdk-go-v2/service/ec2/types"
)

func NewAWSSession(ctx context.Context) (aws.Config, error) {
	cfg, err := config.LoadDefaultConfig(context.Background(), config.WithRegion("ap-northeast-1"))
	if err != nil {
		return aws.Config{}, err
	}
	return cfg, nil
}

func GetAZName(out *ec2.DescribeAvailabilityZonesOutput, id string) string {
	for _, az := range out.AvailabilityZones {
		if *az.ZoneId == id {
			return *az.ZoneName
		}
	}
	return ""
}

func GetPublicIP(ctx context.Context, svc *imds.Client) (string, error) {
	md, err := svc.GetMetadata(ctx, &imds.GetMetadataInput{
		Path: "public-ipv4",
	})
	if err != nil {
		return "", err
	}

	content, _ := io.ReadAll(md.Content)
	return string(content), nil
}

func GetInstanceID(ctx context.Context, svc *imds.Client) (string, error) {
	md, err := svc.GetMetadata(ctx, &imds.GetMetadataInput{
		Path: "instance-id",
	})
	if err != nil {
		return "", err
	}

	content, _ := io.ReadAll(md.Content)
	return string(content), nil
}

func GetVPC(ctx context.Context, svc *imds.Client) (string, error) {
	s, err := net.Interfaces()
	if err != nil {
		return "", err
	}
	var unexpectedNames []string
	for _, i := range s {
		if i.Name == "eth0" || strings.HasPrefix(i.Name, "ens") {
			md, err := svc.GetMetadata(ctx, &imds.GetMetadataInput{
				Path: fmt.Sprintf("network/interfaces/macs/%s/vpc-id", i.HardwareAddr.String()),
			})
			if err != nil {
				return "", err
			}

			content, _ := io.ReadAll(md.Content)
			return string(content), nil
		}
		unexpectedNames = append(unexpectedNames, i.Name)
	}
	return "", fmt.Errorf("no expected network interface (%v)", unexpectedNames)
}

func DescribeInstances(ctx context.Context, svc *ec2.Client, vpc string) ([]*ec2.DescribeInstancesOutput, error) {
	var s []*ec2.DescribeInstancesOutput

	p := ec2.NewDescribeInstancesPaginator(svc, &ec2.DescribeInstancesInput{
		Filters: []types.Filter{
			{
				Name:   aws.String("network-interface.vpc-id"),
				Values: []string{vpc},
			},
		},
	})

	for p.HasMorePages() {
		out, err := p.NextPage(ctx)
		if err != nil {
			return nil, err
		}
		s = append(s, out)
	}

	return s, nil
}

func DescribeVolumes(ctx context.Context, svc *ec2.Client, instances []*ec2.DescribeInstancesOutput) ([]*ec2.DescribeVolumesOutput, error) {
	var s []*ec2.DescribeVolumesOutput

	var instanceIDs []string
	for _, out := range instances {
		for _, res := range out.Reservations {
			for _, i := range res.Instances {
				instanceIDs = append(instanceIDs, *i.InstanceId)
			}
		}
	}

	p := ec2.NewDescribeVolumesPaginator(svc, &ec2.DescribeVolumesInput{
		Filters: []types.Filter{
			{
				Name:   aws.String("attachment.instance-id"),
				Values: instanceIDs,
			},
		},
	})

	for p.HasMorePages() {
		out, err := p.NextPage(ctx)
		if err != nil {
			return nil, err
		}
		s = append(s, out)
	}

	return s, nil
}

func DescribeNetworkInterfaces(ctx context.Context, svc *ec2.Client, vpc string) ([]*ec2.DescribeNetworkInterfacesOutput, error) {
	var s []*ec2.DescribeNetworkInterfacesOutput

	p := ec2.NewDescribeNetworkInterfacesPaginator(svc, &ec2.DescribeNetworkInterfacesInput{
		Filters: []types.Filter{
			{
				Name:   aws.String("vpc-id"),
				Values: []string{vpc},
			},
		},
	})

	for p.HasMorePages() {
		out, err := p.NextPage(ctx)
		if err != nil {
			return nil, err
		}
		s = append(s, out)
	}

	return s, nil
}

func DescribeSecurityGroups(ctx context.Context, svc *ec2.Client, instances []*ec2.DescribeInstancesOutput) ([]*ec2.DescribeSecurityGroupsOutput, error) {
	var s []*ec2.DescribeSecurityGroupsOutput

	var ids []string
	idsUnique := make(map[string]struct{})
	for _, o := range instances {
		for _, r := range o.Reservations {
			for _, i := range r.Instances {
				for _, sg := range i.SecurityGroups {
					if _, ok := idsUnique[*sg.GroupId]; !ok {
						ids = append(ids, *sg.GroupId)
						idsUnique[*sg.GroupId] = struct{}{}
					}
				}
			}
		}
	}

	p := ec2.NewDescribeSecurityGroupsPaginator(svc, &ec2.DescribeSecurityGroupsInput{
		GroupIds: ids,
	})

	for p.HasMorePages() {
		out, err := p.NextPage(ctx)
		if err != nil {
			return nil, err
		}
		s = append(s, out)
	}

	return s, nil
}
