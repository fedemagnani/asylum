interface SpacerProps {
    height: string;
}

const Spacer: React.FC<SpacerProps> = ({ height }) => {
    return <div style={{ height }} />;
};

export default Spacer;