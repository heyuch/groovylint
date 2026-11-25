
        class MyClass {
            /**
             * Return the calculated count of some stuff.
             *
             * @param startIndex - the starting index
             * @return
             * @throws RuntimeException if crazy sh*t happens
             *
             * NOTE: Only the first occurrence of an empty @return tag is found.
             *       So the following line is not flagged as a violation!!!
             * @return
             */
            int countThings(int startIndex) { }

            /**
             *@return
             */
            String otherMethod() { }
        }
        