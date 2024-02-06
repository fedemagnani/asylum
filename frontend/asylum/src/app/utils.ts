
// We define a generic datatype which will be used to store the data from the backend, we also pass as argument a useState function to update the state of the component
const fetchData = async <T,>(path: string, setState: React.Dispatch<React.SetStateAction<T>>) => {
    const response = await fetch(`${process.env.NEXT_PUBLIC_BACKEND_URL}/${path}`);
    const data: T = await response.json();
    setState(data);
}

export {fetchData};