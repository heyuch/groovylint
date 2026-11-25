
            class Person {
                String firstName
                static constraints = {
                    firstName nullable:true
                }
                static mapping = {
                    columns {
                        firstName column: 'First_Name'
                    }
                }
            }
        