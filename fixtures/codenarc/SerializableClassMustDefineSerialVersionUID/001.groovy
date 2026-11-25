
            enum DeferralRequestStatus {    // implements Serializable -- fails
                PENDING(0),
                ACCEPTED(1),
                DENIED(2)
                long value

                DeferralRequestStatus(long pValue) {
                    value = pValue
                }

                long getId() {
                    return value
                }

                String getName() {
                    return GapEnumHelper.getValue(GrailsNameUtils.getShortName(DeferralRequestStatus.class.name), value).name
                }
            }
            