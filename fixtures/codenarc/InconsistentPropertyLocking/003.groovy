
            class Person {
                boolean deceased

                // violation, set method should be locked
                void setDeceased(boolean deceased) {
                    this.deceased = deceased
                }

                @WithReadLock boolean isDeceased() {
                    deceased
                }
            }
        