
            class Person {
                String firstName
                String lastName
                static constraints = {
                    firstName nullable:true
                    lastName nullable:true, maxSize: 30
                }

                static mapping = {
                    firstName column: 'prenom'
                }
            }
        