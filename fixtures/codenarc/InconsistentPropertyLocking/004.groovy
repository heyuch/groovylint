
            class Person {
                boolean parent

                @groovy.transform.WithWriteLock void setParent(boolean parent) {
                    this.parent = parent
                }

                // violation, get method should be locked
                boolean isParent() {
                    parent
                }
            }
        