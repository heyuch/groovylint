
            class Person {
                String firstName
                String lastName
                static constraints = {
                    importFrom Entity, include: ["firstName"]
                    importFrom Entity, include: ["firstName"]
                }
            }
        