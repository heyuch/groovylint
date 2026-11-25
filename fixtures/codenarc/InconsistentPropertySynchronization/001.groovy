
            class Person {
                String name

                synchronized setName(String name) {
                    this.name = name
                }
                // violation, get method should be synchronized
                String getName() {
                    name
                }
            }
        