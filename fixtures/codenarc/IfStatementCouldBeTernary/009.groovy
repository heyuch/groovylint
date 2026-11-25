
            class MyDomain {
                static constraints = {
                    registrationState(nullable: true, validator: { val, obj ->
                        if (val) {
                            return false
                        } else {
                            return true
                        }
                    })
                }
            }
        