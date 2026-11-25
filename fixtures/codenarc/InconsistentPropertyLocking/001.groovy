
            class Person {
                String name

                @WithWriteLock setName(String name) {
                    this.name = name
                }
                // violation, get method should be locked
                String getName() {
                    name
                }
            }
        