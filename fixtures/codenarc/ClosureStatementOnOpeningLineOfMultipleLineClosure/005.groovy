
            def m = [isOkay:{ return true }
            ]
            resource.authenticationDao = [
                getPlans:{ ssn, birthDate -> throw new NoMatchingDataException() },
                isHandler:{     return true}
            ] as AuthenticationDao
            