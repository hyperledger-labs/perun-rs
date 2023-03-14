package remote

import (
	"context"

	"perun.network/go-perun/channel"
)

type FunderService struct {
	funder channel.Funder
}

func NewFunderService(funder channel.Funder) *FunderService {
	return &FunderService{funder: funder}
}

func (f *FunderService) Fund(ctx context.Context, req channel.FundingReq) error {
	return f.funder.Fund(ctx, req)
}
