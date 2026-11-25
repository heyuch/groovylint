
            class Person {
                boolean parent

                synchronized void setParent(boolean parent) {
                    this.parent = parent
                }

                // violation, get method should be synchronized
                boolean isParent() {
                    parent
                }
            }
        