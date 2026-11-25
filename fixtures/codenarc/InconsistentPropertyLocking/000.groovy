
            class Person {
                String name
                Date birthday
                boolean deceased
                boolean parent

                @WithWriteLock setName(String name) {
                    this.name = name
                }
                @groovy.transform.WithReadLock String getName() {
                    name
                }

                void setBirthday(Date birthday) {
                    this.birthday = birthday
                }

                String getBirthday() {
                    birthday
                }

                @groovy.transform.WithWriteLock void setDeceased(boolean deceased) {
                    this.deceased = deceased
                }

                @WithReadLock boolean isDeceased() {
                    deceased
                }

                @WithWriteLock void setParent(boolean parent) {
                    this.parent = parent
                }

                @WithReadLock boolean isParent() {
                    parent
                }
            }
        