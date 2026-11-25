
            class Person {
                String firstName
                String lastName
                static constraints = {
                    importFrom Entity, include: ["firstName"]
                    firstName blank: false
                }
            }
        