
            class Entity {
                String firstName

                static constraints = {
                    firstName size: 0..50, nullable: true
                }
            }

            class Person {
                String firstName
                String lastName
                static constraints = {
                    importFrom Entity
                    lastName maxSize: 30
                }
            }
        