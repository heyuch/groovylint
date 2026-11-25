
            class Person {
                String firstName
                String lastName
                static constraints = {
                    firstName nullable:true, validator:buildValidator(1)
                    lastName nullable:true, validator:buildValidator(2)
                }
            }
        