
            class Person {
                String firstName
                static mapping = {
                    columns {
                        firstName column: 'First_Name'
                    }
                }
                static constraints = {
                    firstName nullable:true
                }
            }
        