
            class Person {
                boolean deceased

                // violation, set method should be synchronized
                void setDeceased(boolean deceased) {
                    this.deceased = deceased
                }

                synchronized boolean isDeceased() {
                    deceased
                }
            }
        