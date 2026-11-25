
            class Person {
                String name
                Date birthday
                boolean deceased
                boolean parent

                synchronized setName(String name) {
                    this.name = name
                }
                synchronized String getName() {
                    name
                }

                void setBirthday(Date birthday) {
                    this.birthday = birthday
                }

                String getBirthday() {
                    birthday
                }

                synchronized void setDeceased(boolean deceased) {
                    this.deceased = deceased
                }

                synchronized boolean isDeceased() {
                    deceased
                }

                synchronized void setParent(boolean parent) {
                    this.parent = parent
                }

                synchronized boolean isParent() {
                    parent
                }
            }
        