
            def filterFunds() {
               (funds.findAll { it.fundCode })
            }
            def filterFunds_TwoExtraParentheses() {
               ((funds.findAll{it.fundCode}))
            }
            def extendFunds() {
               (funds.extend(3) { it.fundCode })
            }
            def purgeFunds() {
               (funds.purge {
                    it.fundCode
                    }
               )
            }
            boolean isSkipParticipant(Participant participant) {
                if(((majorMinorStatusList.findAll{it -> it==(participant.majorStatus + participant.minorStatus).trim()})) ||
                        ((majorStatusList.findAll{it ->it==(participant.majorStatus).trim()})) ||
                        (participant.paymentCode == 1)||(participant.ssn=='')) {
                    return true
                }
                return false
            }

            // The only violation
            def clearFunds() {
               (println(funds.clear('clearing', { it.fundCode })))
            }
          