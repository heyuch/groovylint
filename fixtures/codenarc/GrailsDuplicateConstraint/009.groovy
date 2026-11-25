
            class Person {
                static constraints = {
                    firstName nullable:true
                    lastName nullable:true, maxSize:30
                    firstName nullable:false
                    lastName nullable:false, maxSize:30
                }
            }
        