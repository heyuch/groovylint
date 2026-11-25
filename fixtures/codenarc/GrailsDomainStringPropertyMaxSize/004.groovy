
            class Person {
                String firstName
                String lastName
                static constraints = {
                    firstName nullable:true, size: 0..50
                    lastName nullable:true
                }

                class Address {  }
            }
        