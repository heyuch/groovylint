
            class Person {
                transient String firstName
                static String lastName

                static constraints = {
                    firstName nullable:true
                    lastName nullable:true
                }
            }
        