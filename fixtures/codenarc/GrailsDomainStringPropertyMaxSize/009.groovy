
            class Entity {
                String firstName

                static constraints = {
                    firstName size: 0..50, nullable: true
                    lastName nullable:true, maxSize: 30
                }
            }

            class Person {
                String firstName
                String lastName
                static constraints = {
                    importFrom Entity, include: ["firstName", "lastName"]
                }
            }
        