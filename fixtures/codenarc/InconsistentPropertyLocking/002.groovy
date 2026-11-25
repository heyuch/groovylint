
            class Person {
                Date birthday

                // violation, set method should be locked
                void setBirthday(Date birthday) {
                    this.birthday = birthday
                }

                @groovy.transform.WithReadLock String getBirthday() {
                    birthday
                }
            }
        