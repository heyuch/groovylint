
            def buildCallable() {
                String ssn = 'xxx'
                return new Callable<Object>() {
                    public Object call(){
                        return participantDao.getParticipantPlans(ssn)
                    }
                }
            }
        