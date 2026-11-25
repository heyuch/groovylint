
            static class PingCommand extends PingIntegTest.PingCommand {
                static commandNotAllowedEventReceived
                static restrictionChain

                @Override
                RestrictionChainElement getRestrictionChain() {
                    restrictionChain
                }
            }
        