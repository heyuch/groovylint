
        class MyClass {
            /**
             * Return the calculated count of some stuff.
             *
             * @param name
             *
             * NOTE: Only the first occurrence of an empty @return tag is found.
             *       So the following line is not flagged as a violation!!!
             * @param name2
             */
            int countThings(int startIndex) { }

            /**
             *@param otherName
             */
            String otherMethod() { }
        }
        