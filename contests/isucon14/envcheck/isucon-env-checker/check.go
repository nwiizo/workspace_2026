package main

import (
	"github.com/aws/aws-sdk-go-v2/service/ec2/types"
)

var (
	checkNameByInstanceName = map[string]string{
		"isucon14-contest-1": "contest1",
		"isucon14-contest-2": "contest2",
		"isucon14-contest-3": "contest3",
	}
	privateAddressByInstanceName = map[string]string{
		"isucon14-contest-1": "192.168.0.11",
		"isucon14-contest-2": "192.168.0.12",
		"isucon14-contest-3": "192.168.0.13",
	}
)

func (c *checker) checkAll() {
	c.checkInstances()
	c.checkVolumes()
	c.checkNetworkInterfaces()
	c.checkSecurityGroups()
}

func (c *checker) checkInstances() {
	var count int
	for _, o := range c.DescribeInstances {
		for _, r := range o.Reservations {
			for _, i := range r.Instances {
				count++
				c.checkInstance(&i)
			}
		}
	}
	if count > 3 {
		c.addFailure("%d個の EC2 インスタンスが検出されました (3個である必要があります)", count)
	}
}

func (c *checker) checkInstance(i *types.Instance) {
	id := *i.InstanceId
	if i.InstanceType != c.ExpectedInstanceType {
		c.addFailure("%s のインスタンスタイプが %s です (%sである必要があります)", id, i.InstanceType, c.ExpectedInstanceType)
	}
	if c.ExpectedAMI != "" && *i.ImageId != c.ExpectedAMI {
		c.addFailure("%s の AMI が %s です (%s である必要があります)", id, *i.ImageId, c.ExpectedAMI)
	}
	if c.ExpectedAZ != "" {
		azName := GetAZName(c.DescribeAvailabilityZones, c.ExpectedAZ)
		if *i.Placement.AvailabilityZone != azName {
			c.addFailure("%s のゾーンが %s です (%s (ID: %s) である必要があります)", id, *i.Placement.AvailabilityZone, azName, c.ExpectedAZ)
		}
	}
	if len(i.BlockDeviceMappings) != 1 {
		c.addFailure("%s に %d 個のブロックデバイスが検出されました (1個である必要があります)", id, len(i.BlockDeviceMappings))
	} else if i.BlockDeviceMappings[0].Ebs == nil {
		c.addFailure("%s のブロックデバイスが EBS ではありません", id)
	}
	if len(i.NetworkInterfaces) != 1 {
		c.addFailure("%s に %d 個のネットワークインターフェイスが検出されました (1個である必要があります)", id, len(i.NetworkInterfaces))
	}
}

func (c *checker) checkVolumes() {
	for _, o := range c.DescribeVolumes {
		for _, v := range o.Volumes {
			c.checkVolume(&v)
		}
	}
}

func (c *checker) checkVolume(v *types.Volume) {
	id := *v.VolumeId
	if *v.Size != int32(c.ExpectedVolumeSize) {
		c.addFailure("%s のサイズが %d GB です (%d GB である必要があります)", id, *v.Size, c.ExpectedVolumeSize)
	}
	if v.VolumeType != types.VolumeTypeGp3 {
		c.addFailure("%s のタイプが %s です (gp3 である必要があります)", id, v.VolumeType)
	}
}

func (c *checker) checkNetworkInterfaces() {
	allowedInstances := make(map[string]struct{})
	for _, o := range c.DescribeInstances {
		for _, r := range o.Reservations {
			for _, i := range r.Instances {
				allowedInstances[*i.InstanceId] = struct{}{}
			}
		}
	}

	isAllowed := func(i *types.NetworkInterface) bool {
		if i.Attachment == nil || i.Attachment.InstanceId == nil {
			return false
		}
		_, ok := allowedInstances[*i.Attachment.InstanceId]
		return ok
	}

	for _, o := range c.DescribeNetworkInterfaces {
		for _, i := range o.NetworkInterfaces {
			if !isAllowed(&i) {
				c.addFailure("不明なネットワークインターフェイス (%s) が VPC 内に見つかりました", *i.NetworkInterfaceId)
			}
		}
	}
}

func (c *checker) checkSecurityGroups() {
	for _, out := range c.DescribeSecurityGroups {
		for _, sg := range out.SecurityGroups {
			c.checkSecurityGroup(&sg)
		}
	}

}

func (c *checker) checkSecurityGroup(sg *types.SecurityGroup) {
	id := *sg.GroupId

	var hasIngressSSH, hasIngressHTTPS, hasIngressInternal bool
	for _, p := range sg.IpPermissions {
		if c.isIngressSSH(&p) {
			hasIngressSSH = true
			continue
		}
		if c.isIngressHTTPS(&p) {
			hasIngressHTTPS = true
			continue
		}
		if c.isIngressInternal(&p) {
			hasIngressInternal = true
			continue
		}
		// 不明なルールも許可
	}
	if !hasIngressSSH {
		c.addFailure("%s に SSH を許可するルールがありません", id)
	}
	if !hasIngressHTTPS {
		c.addFailure("%s に HTTPS を許可するルールがありません", id)
	}
	if !hasIngressInternal {
		c.addFailure("%s にインスタンス間通信を許可するルールがありません", id)
	}

	if len(sg.IpPermissionsEgress) != 1 {
		c.addFailure("%s のルールが不正です", id)
	}
	for _, p := range sg.IpPermissionsEgress {
		if !c.isEgressAll(&p) {
			c.addFailure("%s に不正なルールが見つかりました", id)
		}
	}
}

func (c *checker) isIngressSSH(p *types.IpPermission) bool {

	if p.FromPort == nil || *p.FromPort != 22 {
		return false
	}
	if p.ToPort == nil || *p.ToPort != 22 {
		return false
	}
	if p.IpProtocol == nil || *p.IpProtocol != "tcp" {
		return false
	}
	if len(p.IpRanges) != 1 {
		return false
	}
	if p.IpRanges[0].CidrIp == nil || *p.IpRanges[0].CidrIp != "0.0.0.0/0" {
		return false
	}
	if len(p.Ipv6Ranges) != 0 {
		return false
	}
	if len(p.PrefixListIds) != 0 {
		return false
	}
	if len(p.UserIdGroupPairs) != 0 {
		return false
	}

	return true
}

func (c *checker) isIngressHTTPS(p *types.IpPermission) bool {

	if p.FromPort == nil || *p.FromPort != 443 {
		return false
	}
	if p.ToPort == nil || *p.ToPort != 443 {
		return false
	}
	if p.IpProtocol == nil || *p.IpProtocol != "tcp" {
		return false
	}
	if len(p.IpRanges) != 1 {
		return false
	}
	if p.IpRanges[0].CidrIp == nil || *p.IpRanges[0].CidrIp != "0.0.0.0/0" {
		return false
	}
	if len(p.Ipv6Ranges) != 0 {
		return false
	}
	if len(p.PrefixListIds) != 0 {
		return false
	}
	if len(p.UserIdGroupPairs) != 0 {
		return false
	}

	return true
}

func (c *checker) isIngressInternal(p *types.IpPermission) bool {

	if p.FromPort != nil {
		return false
	}
	if p.ToPort != nil {
		return false
	}
	if p.IpProtocol == nil || *p.IpProtocol != "-1" {
		return false
	}
	if len(p.IpRanges) != 1 {
		return false
	}
	if p.IpRanges[0].CidrIp == nil || *p.IpRanges[0].CidrIp != "192.168.0.0/24" {
		return false
	}
	if len(p.Ipv6Ranges) != 0 {
		return false
	}
	if len(p.PrefixListIds) != 0 {
		return false
	}
	if len(p.UserIdGroupPairs) != 0 {
		return false
	}

	return true
}

func (c *checker) isEgressAll(p *types.IpPermission) bool {

	if p.FromPort != nil {
		c.addFailure("送信元 Port が不正です, %v", p.FromPort)
		return false
	}

	if p.ToPort != nil {
		c.addFailure("宛先 Port が不正です, %v", p.ToPort)
		return false
	}

	if p.IpProtocol == nil || *p.IpProtocol != "-1" {
		c.addFailure("IPプロトコル が不正です, %v", p.IpProtocol)
		return false
	}

	if len(p.IpRanges) != 1 {
		c.addFailure("IPレンジが不正です, %v", p.IpRanges)
		return false
	}

	if p.IpRanges[0].CidrIp == nil || *p.IpRanges[0].CidrIp != "0.0.0.0/0" {
		c.addFailure("IPレンジが不正です, %v", p.IpRanges[0].CidrIp)
		return false
	}

	if len(p.Ipv6Ranges) != 0 {
		c.addFailure("不正なIPv6レンジが存在します, %v", p.Ipv6Ranges)
		return false
	}

	if len(p.PrefixListIds) != 0 {
		c.addFailure("不正なPrefixListIDが存在します, %v", p.PrefixListIds)
		return false
	}

	if len(p.UserIdGroupPairs) != 0 {
		c.addFailure("不正なUserIdGroupPairsが存在します, %v", p.UserIdGroupPairs)
		return false
	}

	return true
}
