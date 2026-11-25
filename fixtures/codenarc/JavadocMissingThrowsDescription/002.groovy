
        class MyClass {
            /**
             * Return the calculated count of some stuff.
             *
             * @throws name
             *
             * NOTE: Only the first occurrence of an empty @return tag is found.
             *       So the following line is not flagged as a violation!!!
             * @throws name2
             */
            int countThings(int startIndex) { }

            /**
             *@throws otherName
             */
            String otherMethod() { }
        }
        