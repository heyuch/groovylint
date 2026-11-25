
            class Person {
                Date birthday

                // violation, set method should be synchronized
                void setBirthday(Date birthday) {
                    this.birthday = birthday
                }

                synchronized String getBirthday() {
                    birthday
                }
            }
        